use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore};

// --- NNUE ARCHITECTURE CONFIGURATION ---
// 768 input features (12 piece types * 64 squares).
const INPUT_SIZE: usize = 768;
// Size of the first hidden layer (Feature Transformer).
const L1_SIZE: usize = 256;
// Size of the second hidden layer.
const L2_SIZE: usize = 32;

// ============================================================================
// SIMD BACKEND SELECTION
// ============================================================================
// The hot path of NNUE inference is:
//   1) L1 accumulation: for every occupied square, add a 256-wide i16 row
//      from the weight matrix into the accumulator (saturating).
//   2) CReLU clamp of the 256-wide accumulator.
//   3) L2: a 256x32 mat-vec product (i16 x i16 -> i32).
//   4) L3: a 32-wide dot product.
//
// All four steps are data-parallel over contiguous i16 arrays, which is the
// textbook case for SIMD. We provide:
//   - an AVX2 implementation (x86_64, runtime-detected via CPUID, cached),
//   - a NEON implementation (aarch64, always available on that target),
//   - a portable scalar fallback used anywhere else, or if AVX2 is missing.
//
// Every SIMD routine has a scalar twin computing the identical arithmetic
// (same saturation points, same integer division/clamping), so switching
// backends never changes the evaluation output.
// ============================================================================

#[cfg(target_arch = "x86_64")]
fn has_avx2() -> bool {
    use std::sync::OnceLock;
    static AVX2: OnceLock<bool> = OnceLock::new();
    *AVX2.get_or_init(|| is_x86_feature_detected!("avx2"))
}

/// Main structure for the quantized LunaNNUE neural network.
/// Stores weights and biases as 16-bit integers (`i16`) to optimize performance on CPUs without an FPU.
pub struct LunaNNUE {
    l1_weights: Vec<i16>,
    l1_bias: Vec<i16>,
    l2_weights: Vec<i16>,
    l2_bias: Vec<i16>,
    l3_weights: Vec<i16>,
    l3_bias: Vec<i16>,
}

impl LunaNNUE {
    /// Loads the network's binary weights from an external file in Little-Endian format.
    /// Returns `None` in case of I/O error or corrupted file.
    pub fn load(path: &str) -> Option<Self> {
        let mut file = File::open(path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;

        let mut net = LunaNNUE {
            l1_weights: vec![0; INPUT_SIZE * L1_SIZE],
            l1_bias: vec![0; L1_SIZE],
            l2_weights: vec![0; L1_SIZE * L2_SIZE],
            l2_bias: vec![0; L2_SIZE],
            l3_weights: vec![0; L2_SIZE],
            l3_bias: vec![0; 1],
        };

        let mut offset = 0;
        // Helper closure to read paired bytes sequentially and convert them into i16.
        let mut read_i16 = |count: usize| -> Vec<i16> {
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                if offset + 2 <= buffer.len() {
                    v.push(i16::from_le_bytes([buffer[offset], buffer[offset + 1]]));
                    offset += 2;
                }
            }
            v
        };

        // Ordered mapping of weight and bias vectors.
        net.l1_weights = read_i16(INPUT_SIZE * L1_SIZE);
        net.l1_bias = read_i16(L1_SIZE);
        net.l2_weights = read_i16(L1_SIZE * L2_SIZE);
        net.l2_bias = read_i16(L2_SIZE);
        net.l3_weights = read_i16(L2_SIZE);
        net.l3_bias = read_i16(1);

        // Initial diagnostic check on the binary file status.
        if net.l1_weights.iter().take(100).all(|&x| x == 0) {
            println!("⚠️ WARNING: The NNUE file seems empty!");
        } else {
            #[cfg(target_arch = "x86_64")]
            let backend = if has_avx2() { "AVX2" } else { "scalar" };
            #[cfg(target_arch = "aarch64")]
            let backend = "NEON";
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
            let backend = "scalar";
            println!("✅ NNUE: Weights loaded (Safe mode, {} backend).", backend);
        }

        Some(net)
    }

    /// Performs the feedforward (inference) step of the network for the current position.
    /// Extracts active features via bitboards and propagates values through quantized layers.
    pub fn evaluate(&self, s: &Scacchiera) -> i32 {
        // ==========================================
        // --- LAYER 1: FEATURE TRANSFORMER (QA=255)
        // ==========================================
        // Initialize the L1 accumulator by cloning the initial bias values.
        let mut l1_acc = [0i16; L1_SIZE];
        l1_acc.copy_from_slice(&self.l1_bias);

        // Iterate over the 6 piece types.
        for p_idx in 0..6 {
            // Sub-step for White (Color Index 0).
            let mut bb_w = s.pezzi[p_idx] & s.colori[0];
            while bb_w != 0 {
                let sq = bb_w.trailing_zeros() as usize;
                let offset = (p_idx * 64 + sq) * L1_SIZE;
                simd::accumulate(&mut l1_acc, &self.l1_weights[offset..offset + L1_SIZE]);
                bb_w &= bb_w - 1; // Clear the processed bit (Bit-pop)
            }

            // Sub-step for Black (Color Index 1).
            // Black pieces are shifted by 6 positions in the feature index (+6).
            let mut bb_b = s.pezzi[p_idx] & s.colori[1];
            while bb_b != 0 {
                let sq = bb_b.trailing_zeros() as usize;
                let offset = ((p_idx + 6) * 64 + sq) * L1_SIZE;
                simd::accumulate(&mut l1_acc, &self.l1_weights[offset..offset + L1_SIZE]);
                bb_b &= bb_b - 1;
            }
        }

        // Apply CReLU activation function on Layer 1.
        // The ceiling is limited to 2048, consistent with the QA=255 scale.
        let mut l1_out = [0i16; L1_SIZE];
        simd::crelu(&l1_acc, &mut l1_out, 2048);

        // ==========================================
        // --- LAYER 2: HIDDEN LAYER (QB=64)
        // ==========================================
        let mut l2_acc = [0i32; L2_SIZE];
        simd::forward_l2(&l1_out, &self.l2_weights, &self.l2_bias, &mut l2_acc);

        // CReLU activation function on Layer 2.
        // The ceiling is set to 128 to respect the QB=64 quantization scale.
        let mut l2_out = [0i16; L2_SIZE];
        for i in 0..L2_SIZE {
            l2_out[i] = l2_acc[i].clamp(0, 128) as i16;
        }

        // ==========================================
        // --- LAYER 3: LINEAR OUTPUT
        // ==========================================
        let score = self.l3_bias[0] as i32
            + simd::weighted_sum_div_pow2(&l2_out, &self.l3_weights, 8); // /256 = >>8

        // Synchronize output with the Negamax search perspective.
        if s.turno == Colore::Bianco { score } else { -score }
    }
}

// ============================================================================
// SIMD implementations
// ============================================================================
mod simd {
    use super::{L1_SIZE, L2_SIZE};

    /// acc[i] = saturating_add(acc[i], weights[i]) for i in 0..L1_SIZE
    #[inline(always)]
    pub fn accumulate(acc: &mut [i16; L1_SIZE], weights: &[i16]) {
        debug_assert_eq!(weights.len(), L1_SIZE);

        #[cfg(target_arch = "x86_64")]
        {
            if super::has_avx2() {
                unsafe { x86::accumulate_avx2(acc, weights) };
                return;
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            unsafe { arm::accumulate_neon(acc, weights) };
            return;
        }
        #[allow(unreachable_code)]
        accumulate_scalar(acc, weights);
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn accumulate_scalar(acc: &mut [i16; L1_SIZE], weights: &[i16]) {
        for i in 0..L1_SIZE {
            acc[i] = acc[i].saturating_add(weights[i]);
        }
    }

    /// out[i] = clamp(acc[i], 0, ceiling)
    #[inline(always)]
    pub fn crelu(acc: &[i16; L1_SIZE], out: &mut [i16; L1_SIZE], ceiling: i16) {
        #[cfg(target_arch = "x86_64")]
        {
            if super::has_avx2() {
                unsafe { x86::crelu_avx2(acc, out, ceiling) };
                return;
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            unsafe { arm::crelu_neon(acc, out, ceiling) };
            return;
        }
        #[allow(unreachable_code)]
        crelu_scalar(acc, out, ceiling);
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn crelu_scalar(acc: &[i16; L1_SIZE], out: &mut [i16; L1_SIZE], ceiling: i16) {
        for i in 0..L1_SIZE {
            out[i] = acc[i].clamp(0, ceiling);
        }
    }

    /// acc2[o] = bias[o] + sum_j( l1_out[j] * weights[j * L2_SIZE + o] ) / 2048
    /// (weights are laid out row-major: row j, L2_SIZE contiguous columns)
    #[inline(always)]
    pub fn forward_l2(
        l1_out: &[i16; L1_SIZE],
        weights: &[i16],
        bias: &[i16],
        acc2: &mut [i32; L2_SIZE],
    ) {
        debug_assert_eq!(weights.len(), L1_SIZE * L2_SIZE);
        debug_assert_eq!(bias.len(), L2_SIZE);

        #[cfg(target_arch = "x86_64")]
        {
            if super::has_avx2() && L2_SIZE == 32 {
                unsafe { x86::forward_l2_avx2(l1_out, weights, bias, acc2) };
                return;
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            unsafe { arm::forward_l2_neon(l1_out, weights, bias, acc2) };
            return;
        }
        #[allow(unreachable_code)]
        forward_l2_scalar(l1_out, weights, bias, acc2);
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn forward_l2_scalar(
        l1_out: &[i16; L1_SIZE],
        weights: &[i16],
        bias: &[i16],
        acc2: &mut [i32; L2_SIZE],
    ) {
        for i in 0..L2_SIZE {
            acc2[i] = bias[i] as i32;
        }
        for j in 0..L1_SIZE {
            let lj = l1_out[j] as i32;
            if lj == 0 {
                continue; // CReLU sparsity: skip zeroed activations
            }
            let row = &weights[j * L2_SIZE..j * L2_SIZE + L2_SIZE];
            for i in 0..L2_SIZE {
                acc2[i] += (lj * row[i] as i32) / 2048;
            }
        }
    }

    /// sum_i( truncating_div(a[i] * b[i], 2^shift) )
    ///
    /// NOTE: this deliberately divides *each term* before summing, matching
    /// the reference scalar loop `score += (a[i]*b[i]) / divisor`. Integer
    /// truncating division does not distribute over addition for negative
    /// operands (e.g. (-1)/2 + (-1)/2 == 0 but (-2)/2 == -1), so summing the
    /// raw products first and dividing once would silently change the score.
    #[inline(always)]
    pub fn weighted_sum_div_pow2(a: &[i16], b: &[i16], shift: u32) -> i32 {
        debug_assert_eq!(a.len(), b.len());

        #[cfg(target_arch = "x86_64")]
        {
            if super::has_avx2() && a.len() % 16 == 0 {
                return unsafe { x86::weighted_sum_div_pow2_avx2(a, b, shift) };
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { arm::weighted_sum_div_pow2_neon(a, b, shift) };
        }
        #[allow(unreachable_code)]
        weighted_sum_div_pow2_scalar(a, b, shift)
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn weighted_sum_div_pow2_scalar(a: &[i16], b: &[i16], shift: u32) -> i32 {
        let divisor = 1i32 << shift;
        let mut sum = 0i32;
        for i in 0..a.len() {
            sum += (a[i] as i32 * b[i] as i32) / divisor;
        }
        sum
    }

    // ------------------------------------------------------------------
    // x86_64 / AVX2 backend
    // ------------------------------------------------------------------
    #[cfg(target_arch = "x86_64")]
    mod x86 {
        use super::{L1_SIZE, L2_SIZE};
        use std::arch::x86_64::*;

        #[target_feature(enable = "avx2")]
        pub unsafe fn accumulate_avx2(acc: &mut [i16; L1_SIZE], weights: &[i16]) {
            // 16 x i16 lanes per __m256i -> L1_SIZE / 16 iterations.
            let mut i = 0;
            while i < L1_SIZE {
                let a = _mm256_loadu_si256(acc.as_ptr().add(i) as *const __m256i);
                let w = _mm256_loadu_si256(weights.as_ptr().add(i) as *const __m256i);
                let sum = _mm256_adds_epi16(a, w); // saturating add, matches i16::saturating_add
                _mm256_storeu_si256(acc.as_mut_ptr().add(i) as *mut __m256i, sum);
                i += 16;
            }
        }

        #[target_feature(enable = "avx2")]
        pub unsafe fn crelu_avx2(acc: &[i16; L1_SIZE], out: &mut [i16; L1_SIZE], ceiling: i16) {
            let zero = _mm256_setzero_si256();
            let cap = _mm256_set1_epi16(ceiling);
            let mut i = 0;
            while i < L1_SIZE {
                let a = _mm256_loadu_si256(acc.as_ptr().add(i) as *const __m256i);
                let clamped = _mm256_min_epi16(_mm256_max_epi16(a, zero), cap);
                _mm256_storeu_si256(out.as_mut_ptr().add(i) as *mut __m256i, clamped);
                i += 16;
            }
        }

        /// Truncating (round-toward-zero) division of each i32 lane by 2^shift,
        /// matching Rust's `/` operator for i32 (as opposed to an arithmetic
        /// shift, which rounds toward negative infinity for negative inputs).
        /// Uses the variable-count shift intrinsics since `shift` is a runtime
        /// value here (the immediate-shift intrinsics require a compile-time
        /// constant).
        #[target_feature(enable = "avx2")]
        unsafe fn div_pow2_trunc(x: __m256i, shift: u32) -> __m256i {
            let sign = _mm256_srai_epi32(x, 31); // all-1s if negative, else all-0s (31 is a literal immediate)
            let bias = _mm256_srl_epi32(sign, _mm_cvtsi32_si128((32 - shift) as i32));
            let biased = _mm256_add_epi32(x, bias);
            _mm256_sra_epi32(biased, _mm_cvtsi32_si128(shift as i32))
        }

        /// Outer-product accumulation: for each active (non-zero) input feature j,
        /// broadcast l1_out[j], multiply by the corresponding 32-wide weight row,
        /// divide *each* resulting term by 2048 (matching the scalar reference's
        /// per-term division), then accumulate into a 32-lane i32 accumulator
        /// (4 x __m256i of 8 lanes).
        #[target_feature(enable = "avx2")]
        pub unsafe fn forward_l2_avx2(
            l1_out: &[i16; L1_SIZE],
            weights: &[i16],
            bias: &[i16],
            acc2: &mut [i32; L2_SIZE],
        ) {
            let mut acc0 = _mm256_setzero_si256();
            let mut acc1 = _mm256_setzero_si256();
            let mut acc2v = _mm256_setzero_si256();
            let mut acc3 = _mm256_setzero_si256();

            for j in 0..L1_SIZE {
                let lj = l1_out[j];
                if lj == 0 {
                    continue; // CReLU sparsity: skip zeroed activations
                }
                let broadcast = _mm256_set1_epi32(lj as i32);
                let row_ptr = weights.as_ptr().add(j * L2_SIZE);

                // Widen 4 groups of 8 x i16 -> 8 x i32, multiply, divide, accumulate.
                let w0 = _mm256_cvtepi16_epi32(_mm_loadu_si128(row_ptr as *const __m128i));
                let w1 = _mm256_cvtepi16_epi32(_mm_loadu_si128(row_ptr.add(8) as *const __m128i));
                let w2 = _mm256_cvtepi16_epi32(_mm_loadu_si128(row_ptr.add(16) as *const __m128i));
                let w3 = _mm256_cvtepi16_epi32(_mm_loadu_si128(row_ptr.add(24) as *const __m128i));

                let p0 = div_pow2_trunc(_mm256_mullo_epi32(broadcast, w0), 11);
                let p1 = div_pow2_trunc(_mm256_mullo_epi32(broadcast, w1), 11);
                let p2 = div_pow2_trunc(_mm256_mullo_epi32(broadcast, w2), 11);
                let p3 = div_pow2_trunc(_mm256_mullo_epi32(broadcast, w3), 11);

                acc0 = _mm256_add_epi32(acc0, p0);
                acc1 = _mm256_add_epi32(acc1, p1);
                acc2v = _mm256_add_epi32(acc2v, p2);
                acc3 = _mm256_add_epi32(acc3, p3);
            }

            let mut raw = [0i32; L2_SIZE];
            _mm256_storeu_si256(raw.as_mut_ptr().add(0) as *mut __m256i, acc0);
            _mm256_storeu_si256(raw.as_mut_ptr().add(8) as *mut __m256i, acc1);
            _mm256_storeu_si256(raw.as_mut_ptr().add(16) as *mut __m256i, acc2v);
            _mm256_storeu_si256(raw.as_mut_ptr().add(24) as *mut __m256i, acc3);

            for i in 0..L2_SIZE {
                acc2[i] = bias[i] as i32 + raw[i];
            }
        }

        /// sum_i( truncating_div(a[i]*b[i], 2^shift) ), see the scalar twin
        /// `weighted_sum_div_pow2_scalar` for the exact semantics preserved here.
        #[target_feature(enable = "avx2")]
        pub unsafe fn weighted_sum_div_pow2_avx2(a: &[i16], b: &[i16], shift: u32) -> i32 {
            let mut acc = _mm256_setzero_si256();
            let mut i = 0;
            while i + 16 <= a.len() {
                let va = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
                let vb = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);

                // Widen the low/high 8 x i16 lanes to i32 so each product can be
                // divided individually before summation (matches scalar order).
                let va_lo = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(va));
                let va_hi = _mm256_cvtepi16_epi32(_mm256_extracti128_si256(va, 1));
                let vb_lo = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(vb));
                let vb_hi = _mm256_cvtepi16_epi32(_mm256_extracti128_si256(vb, 1));

                let prod_lo = div_pow2_trunc(_mm256_mullo_epi32(va_lo, vb_lo), shift);
                let prod_hi = div_pow2_trunc(_mm256_mullo_epi32(va_hi, vb_hi), shift);

                acc = _mm256_add_epi32(acc, prod_lo);
                acc = _mm256_add_epi32(acc, prod_hi);
                i += 16;
            }
            let mut sum = hsum_epi32(acc);
            let divisor = 1i32 << shift;
            while i < a.len() {
                sum += (a[i] as i32 * b[i] as i32) / divisor;
                i += 1;
            }
            sum
        }

        #[target_feature(enable = "avx2")]
        unsafe fn hsum_epi32(v: __m256i) -> i32 {
            let hi = _mm256_extracti128_si256(v, 1);
            let lo = _mm256_castsi256_si128(v);
            let sum128 = _mm_add_epi32(hi, lo);
            let hi64 = _mm_unpackhi_epi64(sum128, sum128);
            let sum64 = _mm_add_epi32(sum128, hi64);
            let hi32 = _mm_shuffle_epi32(sum64, 0b01);
            let sum32 = _mm_add_epi32(sum64, hi32);
            _mm_cvtsi128_si32(sum32)
        }
    }

    // ------------------------------------------------------------------
    // aarch64 / NEON backend
    // ------------------------------------------------------------------
    #[cfg(target_arch = "aarch64")]
    mod arm {
        use super::{L1_SIZE, L2_SIZE};
        use std::arch::aarch64::*;

        #[target_feature(enable = "neon")]
        pub unsafe fn accumulate_neon(acc: &mut [i16; L1_SIZE], weights: &[i16]) {
            // 8 x i16 lanes per 128-bit vector -> L1_SIZE / 8 iterations.
            let mut i = 0;
            while i < L1_SIZE {
                let a = vld1q_s16(acc.as_ptr().add(i));
                let w = vld1q_s16(weights.as_ptr().add(i));
                let sum = vqaddq_s16(a, w); // saturating add
                vst1q_s16(acc.as_mut_ptr().add(i), sum);
                i += 8;
            }
        }

        #[target_feature(enable = "neon")]
        pub unsafe fn crelu_neon(acc: &[i16; L1_SIZE], out: &mut [i16; L1_SIZE], ceiling: i16) {
            let zero = vdupq_n_s16(0);
            let cap = vdupq_n_s16(ceiling);
            let mut i = 0;
            while i < L1_SIZE {
                let a = vld1q_s16(acc.as_ptr().add(i));
                let clamped = vminq_s16(vmaxq_s16(a, zero), cap);
                vst1q_s16(out.as_mut_ptr().add(i), clamped);
                i += 8;
            }
        }

        /// Truncating (round-toward-zero) division of each i32 lane by 2^shift,
        /// matching Rust's `/` operator for i32. Uses variable-count shifts
        /// (vshlq_* with a negative amount shifts right) since `shift` is a
        /// runtime value; the `vshrq_n_*` intrinsics require a compile-time
        /// constant immediate.
        #[target_feature(enable = "neon")]
        unsafe fn div_pow2_trunc(x: int32x4_t, shift: i32) -> int32x4_t {
            let sign = vshrq_n_s32(x, 31); // 31 is a literal immediate: all-1s if negative, else all-0s
            let sign_u = vreinterpretq_u32_s32(sign);
            let neg_amt = vdupq_n_s32(-(32 - shift));
            let bias_u = vshlq_u32(sign_u, neg_amt); // negative amount => logical right shift
            let bias = vreinterpretq_s32_u32(bias_u);
            let biased = vaddq_s32(x, bias);
            vshlq_s32(biased, vdupq_n_s32(-shift)) // negative amount => arithmetic right shift
        }

        #[target_feature(enable = "neon")]
        pub unsafe fn forward_l2_neon(
            l1_out: &[i16; L1_SIZE],
            weights: &[i16],
            bias: &[i16],
            acc2: &mut [i32; L2_SIZE],
        ) {
            // 4 lanes of i32 per vector -> L2_SIZE / 4 accumulators.
            let mut accs = [vdupq_n_s32(0); L2_SIZE / 4];

            for j in 0..L1_SIZE {
                let lj = l1_out[j];
                if lj == 0 {
                    continue;
                }
                let broadcast = vdupq_n_s32(lj as i32);
                let row_ptr = weights.as_ptr().add(j * L2_SIZE);

                for k in 0..(L2_SIZE / 4) {
                    let w16 = vld1_s16(row_ptr.add(k * 4));
                    let w32 = vmovl_s16(w16); // widen 4 x i16 -> 4 x i32
                    let prod = vmulq_s32(broadcast, w32);
                    accs[k] = vaddq_s32(accs[k], div_pow2_trunc(prod, 11));
                }
            }

            let mut raw = [0i32; L2_SIZE];
            for k in 0..(L2_SIZE / 4) {
                vst1q_s32(raw.as_mut_ptr().add(k * 4), accs[k]);
            }

            for i in 0..L2_SIZE {
                acc2[i] = bias[i] as i32 + raw[i];
            }
        }

        /// sum_i( truncating_div(a[i]*b[i], 2^shift) ), see the scalar twin
        /// `weighted_sum_div_pow2_scalar` for the exact semantics preserved here.
        #[target_feature(enable = "neon")]
        pub unsafe fn weighted_sum_div_pow2_neon(a: &[i16], b: &[i16], shift: u32) -> i32 {
            let mut acc = vdupq_n_s32(0);
            let mut i = 0;
            let len = a.len();
            while i + 8 <= len {
                let va = vld1q_s16(a.as_ptr().add(i));
                let vb = vld1q_s16(b.as_ptr().add(i));
                let lo = vmull_s16(vget_low_s16(va), vget_low_s16(vb));
                let hi = vmull_s16(vget_high_s16(va), vget_high_s16(vb));
                acc = vaddq_s32(acc, div_pow2_trunc(lo, shift as i32));
                acc = vaddq_s32(acc, div_pow2_trunc(hi, shift as i32));
                i += 8;
            }
            let mut sum = vaddvq_s32(acc);
            let divisor = 1i32 << shift;
            while i < len {
                sum += (a[i] as i32 * b[i] as i32) / divisor;
                i += 1;
            }
            sum
        }
    }
}

// ============================================================================
// Tests: every SIMD backend must match the scalar reference exactly.
// ============================================================================
#[cfg(test)]
mod tests {
    use super::simd::*;
    use super::{L1_SIZE, L2_SIZE};

    fn rand_i16_vec(len: usize, seed: &mut u64) -> Vec<i16> {
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            v.push(((*seed >> 48) as i16) % 2000);
        }
        v
    }

    #[test]
    fn accumulate_matches_scalar() {
        let mut seed = 12345u64;
        let base = rand_i16_vec(L1_SIZE, &mut seed);
        let weights = rand_i16_vec(L1_SIZE, &mut seed);

        let mut acc_simd = [0i16; L1_SIZE];
        acc_simd.copy_from_slice(&base);
        accumulate(&mut acc_simd, &weights);

        let mut acc_scalar = [0i16; L1_SIZE];
        acc_scalar.copy_from_slice(&base);
        for i in 0..L1_SIZE {
            acc_scalar[i] = acc_scalar[i].saturating_add(weights[i]);
        }

        assert_eq!(acc_simd, acc_scalar);
    }

    #[test]
    fn crelu_matches_scalar() {
        let mut seed = 999u64;
        let acc: Vec<i16> = rand_i16_vec(L1_SIZE, &mut seed)
            .iter()
            .map(|&x| x.wrapping_sub(1000))
            .collect();
        let mut acc_arr = [0i16; L1_SIZE];
        acc_arr.copy_from_slice(&acc);

        let mut out_simd = [0i16; L1_SIZE];
        crelu(&acc_arr, &mut out_simd, 2048);

        let mut out_scalar = [0i16; L1_SIZE];
        for i in 0..L1_SIZE {
            out_scalar[i] = acc_arr[i].clamp(0, 2048);
        }

        assert_eq!(out_simd, out_scalar);
    }

    #[test]
    fn forward_l2_matches_scalar() {
        let mut seed = 42u64;
        let l1_out_v: Vec<i16> = rand_i16_vec(L1_SIZE, &mut seed)
            .iter()
            .map(|&x| x.unsigned_abs() as i16 % 2048) // valid CReLU output range
            .collect();
        let mut l1_out = [0i16; L1_SIZE];
        l1_out.copy_from_slice(&l1_out_v);

        let weights = rand_i16_vec(L1_SIZE * L2_SIZE, &mut seed);
        let bias = rand_i16_vec(L2_SIZE, &mut seed);

        let mut acc_simd = [0i32; L2_SIZE];
        forward_l2(&l1_out, &weights, &bias, &mut acc_simd);

        let mut acc_scalar = [0i32; L2_SIZE];
        for i in 0..L2_SIZE {
            acc_scalar[i] = bias[i] as i32;
        }
        for j in 0..L1_SIZE {
            let lj = l1_out[j] as i32;
            if lj == 0 { continue; }
            let row = &weights[j * L2_SIZE..j * L2_SIZE + L2_SIZE];
            for i in 0..L2_SIZE {
                acc_scalar[i] += (lj * row[i] as i32) / 2048;
            }
        }

        assert_eq!(acc_simd, acc_scalar);
    }

    #[test]
    fn weighted_sum_div_pow2_matches_scalar() {
        let mut seed = 777u64;
        // Include plenty of negative values: this is exactly the case where
        // sum-then-divide would diverge from divide-then-sum.
        let a = rand_i16_vec(L2_SIZE, &mut seed);
        let b = rand_i16_vec(L2_SIZE, &mut seed);

        let simd_result = weighted_sum_div_pow2(&a, &b, 8);
        let scalar_result: i32 = a
            .iter()
            .zip(b.iter())
            .map(|(&x, &y)| (x as i32 * y as i32) / 256)
            .sum();

        assert_eq!(simd_result, scalar_result);
    }

    #[test]
    fn weighted_sum_div_pow2_matches_scalar_l1_sized() {
        // Also exercise the L1_SIZE-length case (used nowhere directly today,
        // but confirms the AVX2/NEON remainder-handling loop is correct for
        // lengths that aren't a clean multiple of 16).
        let mut seed = 2024u64;
        let a = rand_i16_vec(37, &mut seed);
        let b = rand_i16_vec(37, &mut seed);

        let simd_result = weighted_sum_div_pow2(&a, &b, 11);
        let scalar_result: i32 = a
            .iter()
            .zip(b.iter())
            .map(|(&x, &y)| (x as i32 * y as i32) / 2048)
            .sum();

        assert_eq!(simd_result, scalar_result);
    }

    #[test]
    fn stress_full_i16_range_many_trials() {
        // Real quantized weights can span the full i16 range and be negative;
        // hammer every primitive with 500 trials to catch rare lane/rounding bugs.
        let mut seed = 0xC0FFEEu64;
        let mut next = |lo: i32, hi: i32| -> i16 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let r = (seed >> 33) as i64;
            (lo as i64 + (r % (hi - lo + 1) as i64)) as i16
        };

        for _ in 0..500 {
            // accumulate
            let base: Vec<i16> = (0..L1_SIZE).map(|_| next(-32000, 32000)).collect();
            let weights: Vec<i16> = (0..L1_SIZE).map(|_| next(i16::MIN as i32, i16::MAX as i32)).collect();
            let mut a1 = [0i16; L1_SIZE];
            a1.copy_from_slice(&base);
            accumulate(&mut a1, &weights);
            let mut a2 = [0i16; L1_SIZE];
            a2.copy_from_slice(&base);
            for i in 0..L1_SIZE { a2[i] = a2[i].saturating_add(weights[i]); }
            assert_eq!(a1, a2, "accumulate mismatch");

            // crelu
            let mut c1 = [0i16; L1_SIZE];
            crelu(&a1, &mut c1, 2048);
            let mut c2 = [0i16; L1_SIZE];
            for i in 0..L1_SIZE { c2[i] = a1[i].clamp(0, 2048); }
            assert_eq!(c1, c2, "crelu mismatch");

            // forward_l2 (c1 is a valid 0..=2048 CReLU output)
            let l2w: Vec<i16> = (0..L1_SIZE * L2_SIZE).map(|_| next(i16::MIN as i32, i16::MAX as i32)).collect();
            let l2b: Vec<i16> = (0..L2_SIZE).map(|_| next(i16::MIN as i32, i16::MAX as i32)).collect();
            let mut l2acc1 = [0i32; L2_SIZE];
            forward_l2(&c1, &l2w, &l2b, &mut l2acc1);
            let mut l2acc2 = [0i32; L2_SIZE];
            for i in 0..L2_SIZE { l2acc2[i] = l2b[i] as i32; }
            for j in 0..L1_SIZE {
                let lj = c1[j] as i32;
                if lj == 0 { continue; }
                for i in 0..L2_SIZE {
                    l2acc2[i] += (lj * l2w[j * L2_SIZE + i] as i32) / 2048;
                }
            }
            assert_eq!(l2acc1, l2acc2, "forward_l2 mismatch");

            // weighted_sum_div_pow2 (L3-style)
            let l2out: Vec<i16> = (0..L2_SIZE).map(|_| next(-32000, 32000)).collect();
            let l3w: Vec<i16> = (0..L2_SIZE).map(|_| next(i16::MIN as i32, i16::MAX as i32)).collect();
            let s1 = weighted_sum_div_pow2(&l2out, &l3w, 8);
            let s2: i32 = l2out.iter().zip(l3w.iter()).map(|(&x, &y)| (x as i32 * y as i32) / 256).sum();
            assert_eq!(s1, s2, "weighted_sum_div_pow2 mismatch");
        }
    }

    #[test]
    #[ignore]
    fn bench_evaluate_throughput() {
        use crate::board::Scacchiera;
        use crate::zobrist::get_zobrist_keys;
        use std::time::Instant;

        let net = super::LunaNNUE::load("luna.nnue").expect("luna.nnue must be present for this bench");
        let z = get_zobrist_keys();
        let board = Scacchiera::new_iniziale(z);

        let iters = 2_000_000u64;
        let start = Instant::now();
        let mut sink = 0i64;
        for _ in 0..iters {
            sink += net.evaluate(&board) as i64;
        }
        let elapsed = start.elapsed();
        println!(
            "evaluate(): {} calls in {:?} ({:.1} ns/call, {:.2} M evals/sec) [sink={}]",
            iters,
            elapsed,
            elapsed.as_nanos() as f64 / iters as f64,
            iters as f64 / elapsed.as_secs_f64() / 1e6,
            sink
        );
    }

    #[test]
    fn full_forward_pass_is_deterministic() {
        let mut seed = 314159u64;
        let l1_out_v: Vec<i16> = rand_i16_vec(L1_SIZE, &mut seed)
            .iter()
            .map(|&x| x.unsigned_abs() as i16 % 2048)
            .collect();
        let mut l1_out = [0i16; L1_SIZE];
        l1_out.copy_from_slice(&l1_out_v);
        let weights = rand_i16_vec(L1_SIZE * L2_SIZE, &mut seed);
        let bias = rand_i16_vec(L2_SIZE, &mut seed);

        let mut a = [0i32; L2_SIZE];
        let mut b = [0i32; L2_SIZE];
        forward_l2(&l1_out, &weights, &bias, &mut a);
        forward_l2(&l1_out, &weights, &bias, &mut b);
        assert_eq!(a, b);
    }
}

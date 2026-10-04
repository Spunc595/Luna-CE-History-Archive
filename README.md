# Luna CE — History Archive

Raw historical source files documenting the development of [Luna Chess Engine](https://github.com/Spunc595/Luna-Chess-Engine), my chess engine, from its first prototype (December 2025) through the version that preceded its current NNUE rewrite (August 2026).

I'm publishing this archive so that anyone questioning the authorship or development history of Luna can check the underlying files directly, rather than take any written account of it on faith. See [HISTORY.md](https://github.com/Spunc595/Luna-Chess-Engine/blob/main/HISTORY.md) in the main repository for the narrative this archive supports.

## What's here

Every folder below is a saved snapshot from my own working copies, organized by the period it belongs to (see HISTORY.md for the reasoning behind each period's dates). Folder and file names are unchanged from the originals.

| Folder | Period | Contents |
|---|---|---|
| `01-javascript-2025-12` | Dec 11–18, 2025 | The first Luna prototype, written in JavaScript (versions 0.1–2.3.1) |
| `02-first-rust-lce-2025-12_2026-01` | Dec 18, 2025 – Jan 12, 2026 | First Rust rewrite; an intensely iterative phase where every small change was saved as its own version |
| `03-rust-continued-2026-01_04` | Jan 12 – Apr 5, 2026 | Continued Rust development, including my first from-scratch NNUE training attempt (`Luna_AI_Lab`) |
| `04-definitiva-2026-07_08` | Jul – Aug 2026 | Dated development snapshots from when I resumed the project, up to the version immediately preceding the current NNUE-rewrite era |
| `05-nnue-training-scripts` | Jan–Feb 2026 | The Python/PyTorch scripts I used for my first self-trained NNUE attempt: PGN data extraction (`train.py`), training (two dated versions, `allena_luna_2026-02-04.py` and the final `allena_luna_2026-02-19_final.py`), export to `.nnue` format (`esporta_*.py` — `esporta_muscolosa.py` is the one I actually used for the definitive network), and evaluation testing (`testa_luna.py`), plus the resulting trained network checkpoints under `trained-networks/` (`luna.nnue`, Feb 19, 2026, is the definitive one) |

## What's excluded

This is a curated subset, not a raw copy of everything on my disk:

- **Source files only** — `.rs`, `.js`, `.py`, `.toml`, `.md`, `.bat`. No compiled binaries, build artifacts, IDE config, training datasets, or third-party tools (chess GUIs, engines used for testing, etc.).
- **One unrelated experiment excluded entirely**: a separate C++ chess engine I wrote (with AI assistance) for a family member's own unrelated project isn't part of Luna's lineage and isn't included here.
- **No credentials of any kind** — I swept this archive for tokens, keys, and secrets before publishing; none were found in the included files, and none were knowingly carried over.

## About the commit dates

Each folder here is committed individually, dated to match its original save date rather than the date it was published to this archive. Dates come from (in order of preference): the original file modification timestamps on my disk, date information encoded in the folder name itself (used where file timestamps were found to have been overwritten by a later bulk-copy operation — noted per-commit where this applies), or, for four folders with no other evidence and that I confirmed predate this archive's construction, an approximate placement noted as such in the commit message.I did this reconstruction with AI assistance (Claude). The source files themselves were developed with AI assistance throughout: the early versions even declare it in their UCI `id author` line ("Daniele & Gemini"), and the later ones were written with Claude. Direction, design decisions and testing are mine. This archive shows the history as it was; it doesn't claim the code was written without help.

## License

These snapshots predate the relicensing: Luna CE was GPLv3 up to v3.1.7 and
is MIT from v4.0.0 (see the engine repository). The snapshots here carry the
GPLv3 license they had at the time.

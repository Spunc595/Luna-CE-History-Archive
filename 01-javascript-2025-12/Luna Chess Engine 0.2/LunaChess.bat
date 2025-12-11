@echo off
chcp 65001 >nul
title ♔ Luna Chess Engine ♚
color 0A
cls

echo ========================================
echo        LUNA CHESS ENGINE v1.0
echo ========================================
echo.
echo Controllo ambiente Node.js...

where node >nul 2>nul
if errorlevel 1 (
    echo ERRORE: Node.js non trovato!
    echo.
    echo INSTALLA Node.js da:
    echo https://nodejs.org/
    echo.
    echo Dopo l'installazione, RIAVVIA il computer
    echo e riprova.
    echo.
    pause
    exit
)

echo Node.js trovato!
echo.
echo Avvio Luna Chess Engine...
echo.
timeout /t 2 /nobreak >nul

node "%~dp0chess_game.js"

if errorlevel 1 (
    echo.
    echo ERRORE nell'esecuzione del gioco.
    echo.
    pause
)

exit
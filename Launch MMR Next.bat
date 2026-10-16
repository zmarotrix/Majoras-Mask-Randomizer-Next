@echo off
rem Launch the MMR Next desktop app in development mode.
rem The first launch after code changes takes a minute to compile.
title MMR Next
cd /d "%~dp0app"

where npm >nul 2>nul
if errorlevel 1 (
    echo Node.js ^(npm^) was not found. Install it from https://nodejs.org and try again.
    pause
    exit /b 1
)

if not exist node_modules (
    echo Installing app dependencies...
    call npm install
    if errorlevel 1 (
        pause
        exit /b 1
    )
)

call npm run tauri dev
if errorlevel 1 pause

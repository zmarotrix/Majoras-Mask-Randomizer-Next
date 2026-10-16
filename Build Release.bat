@echo off
rem Build MMR Next for release on Windows. Everything ends up in the
rem "releases" folder: installers, portable .exe, browser version and CLI.
rem The first run downloads Tauri's installer tools (NSIS and WiX).
title MMR Next release build
cd /d "%~dp0app"

where npm >nul 2>nul
if errorlevel 1 (
    echo Node.js ^(npm^) was not found. Install it from https://nodejs.org and try again.
    pause
    exit /b 1
)
where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust ^(cargo^) was not found. Install it from https://rustup.rs and try again.
    pause
    exit /b 1
)
rustup target list --installed | findstr /x "wasm32-unknown-unknown" >nul
if errorlevel 1 rustup target add wasm32-unknown-unknown
if not exist node_modules (
    echo Installing app dependencies...
    call npm install
    if errorlevel 1 (
        pause
        exit /b 1
    )
)

node scripts\release.mjs
if errorlevel 1 (
    pause
    exit /b 1
)
explorer "%~dp0releases"
pause

@echo off
rem Build the browser version of MMR Next and open it in your web browser.
rem The randomizer runs entirely in the browser; your ROM never leaves it.
rem The first build takes a few minutes; later ones are quick.
title MMR Next (Web)
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
if errorlevel 1 (
    echo Adding Rust's WebAssembly target...
    rustup target add wasm32-unknown-unknown
    if errorlevel 1 (
        pause
        exit /b 1
    )
)

if not exist node_modules (
    echo Installing app dependencies...
    call npm install
    if errorlevel 1 (
        pause
        exit /b 1
    )
)

echo Building the web version...
call npm run build:web
if errorlevel 1 (
    pause
    exit /b 1
)

echo.
echo MMR Next is running at http://localhost:4173
echo Close this window to stop it.
echo.
call npx vite preview --port 4173 --open
if errorlevel 1 pause

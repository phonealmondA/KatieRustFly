@echo off
echo ====================================
echo   KatieFlySimRust Launcher (DEBUG)
echo   FAST BUILD - For Quick Testing!
echo ====================================
echo.

REM Check if Rust is installed
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Cargo not found!
    echo.
    echo Please install Rust from: https://rustup.rs/
    echo.
    pause
    exit /b 1
)

echo Rust/Cargo found!
echo.

REM Navigate to Rust project directory
cd KatieFlySimRust

echo Building in DEBUG mode (much faster!)...
echo Note: Game will run slower than release mode
echo Use this for rapid testing of map changes
echo.

REM Build and run in debug mode (no --release flag = faster builds!)
cargo run

REM Check if cargo run succeeded
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ====================================
    echo   Build/Run Failed!
    echo ====================================
    echo.
    echo Please check the error messages above.
    echo.
    pause
    exit /b 1
)

echo.
echo Game closed successfully!
pause

@echo off
echo Building Katie Map Maker...
cargo build --release
if %errorlevel% neq 0 (
    echo Build failed!
    pause
    exit /b %errorlevel%
)

echo.
echo Starting Katie Map Maker...
echo.
target\release\katie_map_maker.exe

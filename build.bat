@echo off
echo ========================================
echo   CabNet Server - Build Script
echo ========================================
echo.

cd /d "%~dp0"

echo [BUILD] Compiling RELEASE build...
cargo build --release

if %ERRORLEVEL% EQU 0 (
    echo.
    echo [SUCCESS] Build complete!
    echo EXE location: %~dp0target\release\codebar-server.exe
    echo.
) else (
    echo.
    echo [ERROR] Build failed!
    echo.
)

pause

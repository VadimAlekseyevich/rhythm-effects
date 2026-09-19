@echo off
setlocal
cd /d "%~dp0"

echo.
echo [Rhythm Effects] Updating repository...
git pull --rebase origin main
if errorlevel 1 (
    echo.
    echo [ERROR] Git pull failed. Check your internet connection or local Git state.
    echo.
    pause
    exit /b 1
)

echo.
echo [Rhythm Effects] Fetching Rust dependencies...
cargo fetch
if errorlevel 1 goto :cargo_error

echo.
echo [Rhythm Effects] Building workspace...
cargo build --workspace
if errorlevel 1 goto :cargo_error

echo.
echo [Rhythm Effects] Starting application...
cargo run -p rhythm_app
if errorlevel 1 goto :cargo_error

exit /b 0

:cargo_error
echo.
echo [ERROR] Cargo command failed.
echo.
pause
exit /b 1

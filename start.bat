@echo off
rem ============================================================
rem  MicroStep 2.0 - one-click launcher (Windows)
rem  Starts the local server and opens the browser automatically.
rem ============================================================
title MicroStep 2.0
cd /d "%~dp0"

where python >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Python not found in PATH. Please install Python 3.12+ first.
    pause
    exit /b 1
)

rem Open the browser 3s later via a background helper (server needs ~1s to bind)
start "" /min cmd /c "timeout /t 3 /nobreak >nul & start http://127.0.0.1:8765"

echo Starting MicroStep 2.0 ...
echo Server:  http://127.0.0.1:8765   (Ctrl+C or close this window to stop)
echo.
python run.py

echo.
echo Server stopped.
pause

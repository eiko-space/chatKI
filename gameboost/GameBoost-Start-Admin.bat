@echo off
rem Startet GameBoost im Entwicklungsmodus MIT Admin-Rechten (noetig fuer die FPS-Messung).
net session >nul 2>&1
if %errorlevel% neq 0 (
  echo Frage Administratorrechte an ...
  powershell -NoProfile -Command "Start-Process -FilePath \"%~f0\" -Verb RunAs"
  exit /b
)
cd /d "%~dp0"
if not exist node_modules (
  echo Erster Start: installiere Abhaengigkeiten ...
  call npm install
)
call npm run tauri dev
pause

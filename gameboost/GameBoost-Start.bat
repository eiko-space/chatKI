@echo off
rem Startet GameBoost im Entwicklungsmodus (ohne Admin-Rechte).
cd /d "%~dp0"
if not exist node_modules (
  echo Erster Start: installiere Abhaengigkeiten ...
  call npm install
)
call npm run tauri dev
pause

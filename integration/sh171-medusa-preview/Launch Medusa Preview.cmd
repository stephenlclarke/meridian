@echo off
setlocal
set "preview_exe=%~dp0bin\meridian-desktop.exe"
set "preview_dir=%~dp0saves-medusa-preview"
if not exist "%preview_exe%" (
    echo Medusa preview executable is missing. Extract the complete release folder first.
    exit /b 1
)
"%preview_exe%" --medusa-preview-dir "%preview_dir%"
exit /b %ERRORLEVEL%

@echo off
setlocal

set "PROJECT_DIR=%~dp0"
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"

if not exist "%VSWHERE%" (
    echo [ERROR] vswhere.exe not found.
    echo Install Visual Studio 2022 Build Tools with C++ workload:
    echo   winget install Microsoft.VisualStudio.2022.BuildTools
    echo Then open the installer and select "Desktop development with C++".
    pause
    exit /b 1
)

for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VS_INSTALL=%%I"

if not defined VS_INSTALL (
    echo [ERROR] Visual Studio C++ Build Tools not found.
    echo Install workload: "Desktop development with C++".
    pause
    exit /b 1
)

set "VSDEVCMD=%VS_INSTALL%\Common7\Tools\VsDevCmd.bat"
if not exist "%VSDEVCMD%" (
    echo [ERROR] VsDevCmd.bat not found at:
    echo   %VSDEVCMD%
    pause
    exit /b 1
)

call "%VSDEVCMD%" -arch=x64 -host_arch=x64 >nul
if errorlevel 1 (
    echo [ERROR] Failed to initialize MSVC developer environment.
    pause
    exit /b 1
)

cd /d "%PROJECT_DIR%"
echo [INFO] Working directory: %CD%

where cl >nul 2>&1
if errorlevel 1 (
    echo [ERROR] cl.exe is still not in PATH.
    pause
    exit /b 1
)

where link >nul 2>&1
if errorlevel 1 (
    echo [ERROR] link.exe is still not in PATH.
    pause
    exit /b 1
)

for /f "delims=" %%I in ('where cl') do set "CL_PATH=%%I"
for /f "delims=" %%I in ('where link') do (
    set "LINK_PATH=%%I"
    goto :got_link
)

:got_link
echo [INFO] cl.exe: %CL_PATH%
echo [INFO] link.exe: %LINK_PATH%

echo %LINK_PATH% | findstr /i "Git\\usr\\bin\\link.exe" >nul
if not errorlevel 1 (
    echo [ERROR] Incorrect linker detected: %LINK_PATH%
    echo [ERROR] Do not run dx from Git Bash for MSVC builds.
    echo [ERROR] Run this .bat from Explorer, CMD, or PowerShell.
    pause
    exit /b 1
)

set "PROTOC=%PROJECT_DIR%tools\bin\protoc.exe"
if not exist "%PROTOC%" set "PROTOC=C:\tools\bin\protoc.exe"
if not exist "%PROTOC%" (
    for /f "delims=" %%I in ('where protoc 2^>nul') do (
        set "PROTOC=%%I"
        goto :got_protoc
    )
)

:got_protoc
if not exist "%PROTOC%" (
    echo [ERROR] protoc.exe not found.
    echo Install the Windows protobuf compiler and place it at:
    echo   C:\tools\bin\protoc.exe
    echo Or add protoc.exe to PATH before running this script.
    pause
    exit /b 1
)

for %%I in ("%PROTOC%") do set "PROTOC_DIR=%%~dpI"
set "PATH=%PROTOC_DIR%;%PATH%"
echo [INFO] protoc.exe: %PROTOC%

echo [INFO] Starting Overseer...
dx serve --platform desktop
set "EXIT_CODE=%ERRORLEVEL%"

if not "%EXIT_CODE%"=="0" (
    echo [ERROR] dx exited with code %EXIT_CODE%.
    pause
)

exit /b %EXIT_CODE%

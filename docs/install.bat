@echo off
setlocal enabledelayedexpansion

REM Lumesh Installation Script for Windows
REM Downloads from GitHub releases (cross-release assets)
echo Lumesh Installation Script
echo ================================

REM Configuration
set GITHUB_REPO=superiums/lumesh
set INSTALL_DIR=%USERPROFILE%\AppData\Local\Microsoft\WindowsApps
set CONFIG_DIR=%USERPROFILE%\AppData\Roaming\lumesh
set DOC_DIR=%USERPROFILE%\AppData\Local\lumesh

REM Ask for installation type
echo Choose installation type:
echo 1) User installation (recommended)
echo 2) System installation (requires admin)
set /p choice="Enter choice (1-2) [1]: "
if "!choice!"=="" set choice=1

if "!choice!"=="1" (
    set INSTALL_DIR=%USERPROFILE%\AppData\Local\Microsoft\WindowsApps
    set CONFIG_DIR=%USERPROFILE%\AppData\Roaming\lumesh
    set DOC_DIR=%USERPROFILE%\AppData\Local\lumesh
    echo User installation selected
) else if "!choice!"=="2" (
    set INSTALL_DIR=C:\Program Files\lumesh
    set CONFIG_DIR=C:\ProgramData\lumesh
    set DOC_DIR=C:\Program Files\lumesh
    echo System installation selected
    echo Note: This will require administrator privileges
)

REM Get latest version from GitHub
echo Fetching latest version...
for /f "tokens=*" %%i in ('curl -s "https://api.github.com/repos/%GITHUB_REPO%/releases/latest" ^| findstr "tag_name"') do set version_line=%%i
for /f "tokens=2 delims=:," %%v in ("!version_line!") do set LATEST_VERSION=%%v
set LATEST_VERSION=%LATEST_VERSION:"=%
set LATEST_VERSION=%LATEST_VERSION: =%
set LATEST_VERSION=%LATEST_VERSION:v=%
if "%LATEST_VERSION%"=="" (
    echo Failed to fetch latest version
    pause
    exit /b 1
)
echo Latest version: %LATEST_VERSION%

REM Create install directory
if "%choice%"=="2" (
    if not exist "%INSTALL_DIR%" (
        echo Creating system directory...
        mkdir "%INSTALL_DIR%" 2>nul || (
            echo Failed to create directory. Run as administrator?
            pause
            exit /b 1
        )
    )
) else (
    if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
)

REM Download binary from GitHub (cross-release:c%LATEST_VERSION%)
echo Downloading lume from GitHub...
curl -L -o "%INSTALL_DIR%\lume.exe" "https://github.com/%GITHUB_REPO%/releases/download/c%LATEST_VERSION%/lume-x86_64-pc-windows-gnu.exe"

REM Download lume-se and doc are not in cross-release, skip
echo lume-se is not included in this release.
echo To use lume-se, build with --features runner.

REM Documentation
echo Downloading documentation...
if not exist "%DOC_DIR%" mkdir "%DOC_DIR%"
curl -L -o "%TEMP%\data.tgz" "https://github.com/%GITHUB_REPO%/releases/download/c%LATEST_VERSION%/data.tgz"
if exist "%TEMP%\data.tgz" (
    tar -xzf "%TEMP%\data.tgz" -C "%DOC_DIR%"
    del "%TEMP%\data.tgz"
    echo Documentation extracted to: %DOC_DIR%
) else (
    echo Documentation not available for this release.
)

echo.
echo Installation completed successfully!
echo Installation location: %INSTALL_DIR%
echo To start using Lumesh:
echo   Interactive shell: lume
echo.
echo Note: Add %INSTALL_DIR% to your PATH if not already present
pause

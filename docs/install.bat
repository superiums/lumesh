@echo off
setlocal enabledelayedexpansion

REM Lumesh Installation Script for Windows
echo Lumesh Installation Script
echo ================================

REM Configuration
set CODEBERG_REPO=santo/lumesh
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

REM Platform detection
echo Detected platform: Windows

REM Get latest version
echo Fetching latest version...
for /f "tokens=*" %%i in ('curl -s "https://codeberg.org/api/v1/repos/%CODEBERG_REPO%/releases/latest" ^| findstr "tag_name"') do set version_line=%%i
set LATEST_VERSION=%version_line:~16,-2%
if "%LATEST_VERSION%"=="" (
    echo Failed to fetch latest version
    pause
    exit /b 1
)
echo Latest version: %LATEST_VERSION%

REM Create install directory
if "!choice!"=="2" (
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

REM Download binaries
echo Downloading lume from Codeberg...
curl -L -o "%INSTALL_DIR%\lume.exe" "https://codeberg.org/%CODEBERG_REPO%/releases/download/v%LATEST_VERSION%/lume-windows.exe"

echo Downloading lume-se from Codeberg...
curl -L -o "%INSTALL_DIR%\lume-se.exe" "https://codeberg.org/%CODEBERG_REPO%/releases/download/v%LATEST_VERSION%/lume-se-windows.exe"

REM Create symlink (Windows junction)
echo Creating symlink from lume-se to lumesh...
if exist "%INSTALL_DIR%\lumesh.exe" del "%INSTALL_DIR%\lumesh.exe"
mklink "%INSTALL_DIR%\lumesh.exe" "%INSTALL_DIR%\lume.exe" >nul 2>&1

REM Download and extract documentation
echo Downloading documentation...
if not exist "%DOC_DIR%" mkdir "%DOC_DIR%"
curl -L -o "%TEMP%\doc.tar.gz" "https://codeberg.org/%CODEBERG_REPO%/releases/download/v%LATEST_VERSION%/doc.tar.gz"
tar -xzf "%TEMP%\doc.tar.gz" -C "%TEMP%"
xcopy "%TEMP%\doc\install\*" "%DOC_DIR%\" /E /Y >nul
del "%TEMP%\doc.tar.gz"
rmdir /S /Q "%TEMP%\doc" 2>nul

echo.
echo Installation completed successfully!
echo Installation location: %INSTALL_DIR%
echo To start using Lumesh:
echo   Interactive shell: lume
echo   Script execution: lume-se script.lm
echo   Documentation: %DOC_DIR%
echo.
echo Online documentation: https://lumesh.codeberg.page
echo.
echo Note: Add %INSTALL_DIR% to your PATH if not already present
pause

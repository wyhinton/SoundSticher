@echo off
setlocal enabledelayedexpansion

:: Sound Stitch Release Script for Windows
:: This script helps you create and publish releases easily

title Sound Stitch Release Manager

:header
echo.
echo ================================
echo 🎵 Sound Stitch Release Manager
echo ================================
echo.

:: Check if we're in a git repository
git rev-parse --git-dir >nul 2>&1
if errorlevel 1 (
    echo ❌ This script must be run from a git repository
    pause
    exit /b 1
)

:: Check if working directory is clean
git status --porcelain 2>nul | findstr . >nul
if not errorlevel 1 (
    echo ⚠️ Working directory is not clean
    echo Uncommitted changes:
    git status --short
    echo.
    set /p continue="Do you want to continue anyway? (y/N): "
    if /i not "!continue!"=="y" (
        echo ℹ️ Release cancelled
        pause
        exit /b 0
    )
)

:: Get current version from package.json
set current_version=0.0.0
if exist package.json (
    for /f "tokens=2 delims=:, " %%i in ('findstr "version" package.json') do (
        set version_line=%%i
        set current_version=!version_line:"=!
    )
)

:menu
echo Current version: !current_version!
echo.
echo Select release type:
echo 1) Patch release (bug fixes)
echo 2) Minor release (new features)
echo 3) Major release (breaking changes)
echo 4) Custom version
echo 5) Show current status
echo 6) Exit
echo.
set /p choice="Choose an option (1-6): "

if "!choice!"=="1" (
    call :calculate_patch_version !current_version! new_version
    echo ℹ️ Creating patch release: !current_version! → !new_version!
    goto :confirm_release
)
if "!choice!"=="2" (
    call :calculate_minor_version !current_version! new_version
    echo ℹ️ Creating minor release: !current_version! → !new_version!
    goto :confirm_release
)
if "!choice!"=="3" (
    call :calculate_major_version !current_version! new_version
    echo ℹ️ Creating major release: !current_version! → !new_version!
    goto :confirm_release
)
if "!choice!"=="4" (
    set /p new_version="Enter custom version (e.g., 1.2.3): "
    call :validate_version !new_version!
    if errorlevel 1 goto :menu
    echo ℹ️ Creating custom release: !current_version! → !new_version!
    goto :confirm_release
)
if "!choice!"=="5" (
    echo ℹ️ Current project status:
    git branch --show-current > temp_branch.txt
    set /p current_branch=<temp_branch.txt
    del temp_branch.txt
    echo   Version: !current_version!
    echo   Branch: !current_branch!
    git log -1 --pretty=format:"  Last commit: %%h - %%s (%%an, %%ar)"
    echo.
    echo.
    goto :menu
)
if "!choice!"=="6" (
    echo ℹ️ Goodbye!
    pause
    exit /b 0
)

echo ❌ Invalid option. Please choose 1-6.
goto :menu

:confirm_release
echo.
echo ⚠️ This will:
echo   • Update version in package.json, Cargo.toml, and tauri.conf.json
echo   • Create a git commit with the version bump
echo   • Create and push a git tag (v!new_version!)
echo   • Trigger GitHub Actions to build and release for all platforms
echo.
set /p confirm="Continue with release !new_version!? (y/N): "
if /i not "!confirm!"=="y" (
    echo ℹ️ Release cancelled
    goto :menu
)

call :update_version !new_version!
call :create_and_push_tag !new_version!
call :show_release_status !new_version!
pause
exit /b 0

:calculate_patch_version
set version=%1
for /f "tokens=1,2,3 delims=." %%a in ("%version%") do (
    set /a patch=%%c+1
    set %2=%%a.%%b.!patch!
)
exit /b 0

:calculate_minor_version
set version=%1
for /f "tokens=1,2,3 delims=." %%a in ("%version%") do (
    set /a minor=%%b+1
    set %2=%%a.!minor!.0
)
exit /b 0

:calculate_major_version
set version=%1
for /f "tokens=1,2,3 delims=." %%a in ("%version%") do (
    set /a major=%%a+1
    set %2=!major!.0.0
)
exit /b 0

:validate_version
set version=%1
echo %version% | findstr /r "^[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*$" >nul
if errorlevel 1 (
    echo ❌ Invalid version format. Use semantic versioning (e.g., 1.0.0)
    exit /b 1
)
exit /b 0

:update_version
set version=%1
echo ℹ️ Updating version to %version% in all files...

:: Update package.json
if exist package.json (
    powershell -Command "(Get-Content package.json) -replace '\"version\": \".*\"', '\"version\": \"%version%\"' | Set-Content package.json"
    echo ✅ Updated package.json
)

:: Update Cargo.toml
if exist src-tauri\Cargo.toml (
    powershell -Command "(Get-Content src-tauri\Cargo.toml) -replace 'version = \".*\"', 'version = \"%version%\"' | Set-Content src-tauri\Cargo.toml"
    echo ✅ Updated src-tauri\Cargo.toml
)

:: Update tauri.conf.json
if exist src-tauri\tauri.conf.json (
    powershell -Command "(Get-Content src-tauri\tauri.conf.json) -replace '\"version\": \".*\"', '\"version\": \"%version%\"' | Set-Content src-tauri\tauri.conf.json"
    echo ✅ Updated src-tauri\tauri.conf.json
)
exit /b 0

:create_and_push_tag
set version=%1
set tag=v%version%

echo ℹ️ Creating git tag %tag%...

:: Add updated files
git add package.json src-tauri\Cargo.toml src-tauri\tauri.conf.json

:: Commit version changes
git commit -m "bump version to %version%" 2>nul

:: Create annotated tag
git tag -a "%tag%" -m "Release %tag%"

echo ℹ️ Pushing changes and tag to remote...
git push origin main 2>nul || git push origin master 2>nul
git push origin "%tag%"

echo ✅ Tag %tag% created and pushed
exit /b 0

:show_release_status
set version=%1
set tag=v%version%

echo.
echo ✅ Release %tag% has been initiated!
echo.
echo ℹ️ What happens next:
echo   1. GitHub Actions will build your app for all platforms
echo   2. Binaries will be automatically uploaded to the release
echo   3. You can monitor progress in GitHub Actions
echo.
echo ℹ️ The release will be available in your GitHub repository's releases section
echo.
exit /b 0

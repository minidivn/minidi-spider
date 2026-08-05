@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

:: ===========================================================================
:: MiniDi Spider — Run Incremental Fill + Push to GitHub
:: ===========================================================================
:: Usage:
::   double-click or run from cmd:
::     examples\run-all.bat              (run next partition via cursor)
::     examples\run-all.bat hist-tran    (specific partition)
::     examples\run-all.bat --list       (list partitions)
::     examples\run-all.bat --reset      (reset cursor)
::     examples\run-all.bat --all        (run ALL 50 partitions sequentially)
::     examples\run-all.bat --push       (push data repo to GitHub)
:: ===========================================================================

set SPIDER=G:\i2c\PROJECTS\MiniPlatform\MiniTools\MinidiSpider
set DATA_REPO=G:\i2c\PROJECTS\MiniPlatform\MiniDi\Data\minidi-vn-data
set SCRIPT=%SPIDER%\examples\local-incremental.sh

:: ── List partitions ───────────────────────────────────────────────────────
if "%1"=="--list" (
    wsl -- bash -c "cd '%SPIDER%' && cargo run --release -- crawl --list-partitions"
    exit /b
)

:: ── Reset cursor ──────────────────────────────────────────────────────────
if "%1"=="--reset" (
    copy /y "%SPIDER%\examples\cursor.seed.json" "%DATA_REPO%\_data\cursor.json"
    echo Cursor reset to start (adm-vn-country)
    exit /b
)

:: ── Push to GitHub ────────────────────────────────────────────────────────
if "%1"=="--push" (
    cd /d "%DATA_REPO%"
    echo Pushing to minidivn/minidi-vn-data...
    git remote add origin https://github.com/minidivn/minidi-vn-data.git 2>nul
    git push -u origin main
    echo Done.
    exit /b
)

:: ── Run ALL 50 partitions ─────────────────────────────────────────────────
if "%1"=="--all" (
    echo Running all 50 partitions sequentially...
    echo This will take a long time (each partition ~2-30 seconds of SPARQL).
    echo.
    for /l %%i in (1,1,50) do (
        call :run_next_partition
        if not !PARTITION!=="" (
            echo [%%i/50] Running !PARTITION!...
            wsl -- bash "%SCRIPT%" !PARTITION!
        )
    )
    echo All done!
    exit /b
)

:: ── Run single partition ──────────────────────────────────────────────────
wsl -- bash "%SCRIPT%" %1
exit /b

:run_next_partition
    for /f "usebackq delims=" %%p in (`powershell -Command "try{$c=Get-Content '%DATA_REPO%\_data\cursor.json'|ConvertFrom-Json;echo $c.current_partition}catch{echo adm-vn-country}"`) do set PARTITION=%%p
    goto :eof

$ErrorActionPreference = "Stop"

Set-Location $PSScriptRoot

Write-Host "Checking prerequisites..." -ForegroundColor Cyan

if (-not (Get-Command "7z" -ErrorAction SilentlyContinue)) {
    Write-Error "7-Zip (7z) not found in PATH! Please install 7-Zip and add it to your PATH to extract .7z files."
    Read-Host "Press Enter to exit..."
    exit 1
}

$targetDir = Join-Path "src-tauri" "lib"
$tempDir = Join-Path $targetDir "temp"

if (-not (Test-Path $tempDir)) {
    Write-Host "Creating working directory: $tempDir" -ForegroundColor Green
    New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
}

# ==========================================
# Task 1: Download & Setup libmpv-wrapper
# ==========================================
Write-Host "`n[1/2] Processing libmpv-wrapper..." -ForegroundColor Cyan

$wrapperBaseUrl = "https://github.com/nini22P/libmpv-wrapper/releases/latest/download"
$wrapperShaUrl = "$wrapperBaseUrl/sha256.txt"

try {
    Write-Host "  Fetching metadata..."
    $content = Invoke-RestMethod -Uri $wrapperShaUrl -UseBasicParsing
    
    $line = $content -split "`r?`n" | Where-Object { $_ -like "*libmpv-wrapper-windows-x86_64*" } | Select-Object -First 1
    
    if (-not $line) { throw "Could not find 'libmpv-wrapper-windows-x86_64' in SHA256 file." }
    
    $parts = $line -split '\s+'
    $wrapperFileName = $parts[-1].Trim()
    
    Write-Host "  Found version: $wrapperFileName" -ForegroundColor Yellow
    
    $wrapperZip = Join-Path $tempDir "wrapper_temp.zip"
    Write-Host "  Downloading to temp..."
    Invoke-WebRequest -Uri "$wrapperBaseUrl/$wrapperFileName" -OutFile $wrapperZip
    
    $wrapperExtractDir = Join-Path $tempDir "wrapper_extract"
    if (Test-Path $wrapperExtractDir) { Remove-Item -Recurse -Force $wrapperExtractDir }
    Expand-Archive -Path $wrapperZip -DestinationPath $wrapperExtractDir -Force
    
    $wrapperDll = Get-ChildItem -Path $wrapperExtractDir -Filter "libmpv-wrapper.dll" -Recurse | Select-Object -First 1
    
    if ($wrapperDll) {
        Move-Item -Path $wrapperDll.FullName -Destination "$targetDir\libmpv-wrapper.dll" -Force
        Write-Host "  -> libmpv-wrapper.dll downloaded." -ForegroundColor Green
    } else {
        throw "libmpv-wrapper.dll not found in the downloaded archive."
    }

} catch {
    Write-Error "Failed to process libmpv-wrapper: $_"
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
    Read-Host "Press Enter to exit..."
    exit 1
}

# ==========================================
# Task 2: Download & Setup libmpv (zhongfly)
# ==========================================
Write-Host "`n[2/2] Processing libmpv..." -ForegroundColor Cyan

$mpvBaseUrl = "https://github.com/zhongfly/mpv-winbuild/releases/latest/download"
$mpvShaUrl = "$mpvBaseUrl/sha256.txt"

try {
    Write-Host "  Fetching metadata..."
    $content = Invoke-RestMethod -Uri $mpvShaUrl -UseBasicParsing

    $line = $content -split "`r?`n" | Where-Object { $_ -like "*mpv-dev-lgpl-x86_64*" -and $_ -notlike "*v3*" } | Select-Object -First 1
    
    if (-not $line) { throw "Could not find 'mpv-dev-lgpl-x86_64' in SHA256 file." }
    
    $parts = $line -split '\s+'
    $mpvFileName = $parts[-1].Trim()
    
    Write-Host "  Found version: $mpvFileName" -ForegroundColor Yellow
    
    $mpvArchive = Join-Path $tempDir "libmpv_temp.7z"
    Write-Host "  Downloading (this file is large)..."
    Invoke-WebRequest -Uri "$mpvBaseUrl/$mpvFileName" -OutFile $mpvArchive
    
    $mpvExtractDir = Join-Path $tempDir "libmpv_extract"
    if (Test-Path $mpvExtractDir) { Remove-Item -Recurse -Force $mpvExtractDir }
    
    Write-Host "  Extracting with 7-Zip..."
    $null = Start-Process "7z" -ArgumentList "x `"$mpvArchive`" -o`"$mpvExtractDir`" -y" -Wait -NoNewWindow -PassThru
    
    $mpvDll = Get-ChildItem -Path $mpvExtractDir -Filter "libmpv-2.dll" -Recurse | Select-Object -First 1
    
    if ($mpvDll) {
        Move-Item -Path $mpvDll.FullName -Destination "$targetDir\libmpv-2.dll" -Force
        Write-Host "  -> libmpv-2.dll downloaded." -ForegroundColor Green
    } else {
        throw "libmpv-2.dll not found in the downloaded archive."
    }

} catch {
    Write-Error "Failed to process libmpv: $_"
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
    Read-Host "Press Enter to exit..."
    exit 1
}

# ==========================================
# Cleanup & Finish
# ==========================================
Write-Host "`nCleaning up temporary files..." -ForegroundColor Gray
if (Test-Path $tempDir) {
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
}

Write-Host "`n-----------------------------------------------------"
Write-Host "SUCCESS! All libraries are set up in src-tauri\lib" -ForegroundColor Green
Write-Host "-----------------------------------------------------"

Read-Host "Press Enter to close..."
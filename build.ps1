# CabNet Server - Build Script
# Compiles the application into a release EXE

param(
    [switch]$Debug,
    [switch]$Run,
    [switch]$Clean
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  CabNet Server - Build Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Ensure cargo is in PATH
$env:Path += ";$env:USERPROFILE\.cargo\bin"

# Check if Rust is installed
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Host "[ERROR] Cargo not found. Please run install-requirements.ps1 first." -ForegroundColor Red
    exit 1
}

# Get script directory
$projectDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectDir

# Clean build if requested
if ($Clean) {
    Write-Host "[CLEAN] Cleaning previous build artifacts..." -ForegroundColor Yellow
    cargo clean 2>&1 | Out-Null
    Write-Host "  [OK] Clean complete" -ForegroundColor Green
    Write-Host ""
}

# Determine build type
if ($Debug) {
    Write-Host "[BUILD] Compiling DEBUG build..." -ForegroundColor Yellow
    $buildArgs = "build"
    $outputDir = "target\debug"
} else {
    Write-Host "[BUILD] Compiling RELEASE build (optimized)..." -ForegroundColor Yellow
    $buildArgs = "build --release"
    $outputDir = "target\release"
}

# Run the build
Write-Host ""
$startTime = Get-Date
$buildOutput = Invoke-Expression "cargo $buildArgs 2>&1"
$endTime = Get-Date
$buildTime = ($endTime - $startTime).TotalSeconds

# Check build result
if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  Build Successful!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "Build time: $([math]::Round($buildTime, 2)) seconds" -ForegroundColor Gray
    Write-Host ""
    
    $exePath = Join-Path $projectDir "$outputDir\codebar-server.exe"
    
    if (Test-Path $exePath) {
        $fileInfo = Get-Item $exePath
        $fileSizeMB = [math]::Round($fileInfo.Length / 1MB, 2)
        
        Write-Host "Output:" -ForegroundColor White
        Write-Host "  Location: $exePath" -ForegroundColor Cyan
        Write-Host "  Size: $fileSizeMB MB" -ForegroundColor Gray
        Write-Host ""
        
        # Copy to root directory for easy access
        $rootExe = Join-Path $projectDir "CabNet-Server.exe"
        Copy-Item $exePath $rootExe -Force
        Write-Host "  Copied to: $rootExe" -ForegroundColor Green
        Write-Host ""
        
        # Run if requested
        if ($Run) {
            Write-Host "[RUN] Starting CabNet Server..." -ForegroundColor Yellow
            Write-Host ""
            Start-Process $rootExe
        } else {
            Write-Host "Run with: .\CabNet-Server.exe" -ForegroundColor Gray
            Write-Host "Or use:   .\build.ps1 -Run" -ForegroundColor Gray
        }
    }
} else {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Red
    Write-Host "  Build Failed!" -ForegroundColor Red  
    Write-Host "========================================" -ForegroundColor Red
    Write-Host ""
    Write-Host $buildOutput -ForegroundColor Red
    exit 1
}

Write-Host ""

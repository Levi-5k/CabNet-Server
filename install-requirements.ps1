# CabNet Server - Requirements Installation Script
# Run this script in PowerShell as Administrator

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  CabNet Server - Requirements Setup" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "[WARNING] Not running as Administrator. Some installations may fail." -ForegroundColor Yellow
    Write-Host "Consider rerunning with: Start-Process powershell -Verb runAs -ArgumentList '-File', '$PSCommandPath'" -ForegroundColor Yellow
    Write-Host ""
}

# Function to check if a command exists
function Test-Command($cmdname) {
    return [bool](Get-Command -Name $cmdname -ErrorAction SilentlyContinue)
}

# 1. Check/Install Rust
Write-Host "[1/3] Checking Rust installation..." -ForegroundColor Yellow
if (Test-Command "rustc") {
    $rustVersion = rustc --version
    Write-Host "  [OK] Rust is installed: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "  [INSTALLING] Rust not found. Downloading rustup..." -ForegroundColor Yellow
    
    # Download and run rustup-init
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupPath = "$env:TEMP\rustup-init.exe"
    
    try {
        Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath -UseBasicParsing
        Write-Host "  [INSTALLING] Running rustup installer..." -ForegroundColor Yellow
        Start-Process -FilePath $rustupPath -ArgumentList "-y" -Wait
        
        # Refresh PATH
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
        $env:Path += ";$env:USERPROFILE\.cargo\bin"
        
        Write-Host "  [OK] Rust installed successfully!" -ForegroundColor Green
    } catch {
        Write-Host "  [ERROR] Failed to install Rust: $_" -ForegroundColor Red
        Write-Host "  Please install manually from https://rustup.rs" -ForegroundColor Red
    }
}

# 2. Update Rust toolchain
Write-Host ""
Write-Host "[2/3] Updating Rust toolchain..." -ForegroundColor Yellow
if (Test-Command "rustup") {
    rustup update stable 2>&1 | Out-Null
    Write-Host "  [OK] Rust toolchain updated" -ForegroundColor Green
} else {
    Write-Host "  [SKIP] rustup not available" -ForegroundColor Yellow
}

# 3. Install required Rust targets (for Windows)
Write-Host ""
Write-Host "[3/3] Ensuring Windows target is installed..." -ForegroundColor Yellow
if (Test-Command "rustup") {
    rustup target add x86_64-pc-windows-msvc 2>&1 | Out-Null
    Write-Host "  [OK] Windows MSVC target ready" -ForegroundColor Green
}

# Summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Installation Complete!" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Display versions
Write-Host "Installed versions:" -ForegroundColor White
if (Test-Command "rustc") {
    Write-Host "  Rust: $(rustc --version)" -ForegroundColor Gray
}
if (Test-Command "cargo") {
    Write-Host "  Cargo: $(cargo --version)" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Next step: Run .\build.ps1 to compile the application" -ForegroundColor Green
Write-Host ""

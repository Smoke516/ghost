# Ghost SSH Manager Installation Script
# For Windows PowerShell

param(
    [string]$InstallDir = "$env:USERPROFILE\.local\bin"
)

# Configuration
$REPO = "username/ghost"
$BINARY_NAME = "ghost.exe"

# Set error action preference
$ErrorActionPreference = "Stop"

# Functions for colored output
function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "[SUCCESS] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
    exit 1
}

# Check if running as administrator
function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# Get latest release version
function Get-LatestVersion {
    Write-Info "Fetching latest release information..."
    
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
        $version = $response.tag_name
        
        if ([string]::IsNullOrEmpty($version)) {
            Write-Error "Could not determine latest version"
        }
        
        Write-Info "Latest version: $version"
        return $version
    }
    catch {
        Write-Error "Failed to fetch release information: $_"
    }
}

# Download and install binary
function Install-Binary {
    param([string]$Version)
    
    $downloadUrl = "https://github.com/$REPO/releases/download/$Version/ghost-windows-x64.zip"
    $tempDir = [System.IO.Path]::GetTempPath()
    $archivePath = Join-Path $tempDir "ghost.zip"
    $extractPath = Join-Path $tempDir "ghost_extract"
    
    Write-Info "Downloading from: $downloadUrl"
    
    try {
        # Download the archive
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
        
        if (-not (Test-Path $archivePath)) {
            Write-Error "Download failed"
        }
        
        Write-Info "Extracting archive..."
        
        # Extract the archive
        if (Test-Path $extractPath) {
            Remove-Item -Path $extractPath -Recurse -Force
        }
        Expand-Archive -Path $archivePath -DestinationPath $extractPath
        
        # Create install directory
        if (-not (Test-Path $InstallDir)) {
            New-Item -Path $InstallDir -ItemType Directory -Force | Out-Null
        }
        
        # Find and move the binary
        $binaryPath = Get-ChildItem -Path $extractPath -Name $BINARY_NAME -Recurse | Select-Object -First 1
        if (-not $binaryPath) {
            Write-Error "Binary not found in archive"
        }
        
        $sourcePath = Join-Path $extractPath $binaryPath
        $targetPath = Join-Path $InstallDir $BINARY_NAME
        
        Copy-Item -Path $sourcePath -Destination $targetPath -Force
        
        # Clean up
        Remove-Item -Path $archivePath -Force -ErrorAction SilentlyContinue
        Remove-Item -Path $extractPath -Recurse -Force -ErrorAction SilentlyContinue
        
        Write-Success "Ghost SSH Manager installed to $targetPath"
    }
    catch {
        Write-Error "Installation failed: $_"
    }
}

# Check and update PATH
function Update-Path {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
    
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Warning "Install directory $InstallDir is not in your PATH"
        
        try {
            $newPath = "$currentPath;$InstallDir"
            [Environment]::SetEnvironmentVariable("Path", $newPath, [EnvironmentVariableTarget]::User)
            Write-Success "Added $InstallDir to your PATH"
            Write-Info "Please restart your terminal or PowerShell session for PATH changes to take effect"
        }
        catch {
            Write-Warning "Could not automatically update PATH. Please add $InstallDir to your PATH manually."
            Write-Info "You can do this through System Properties > Environment Variables"
        }
    }
}

# Test installation
function Test-Installation {
    $binaryPath = Join-Path $InstallDir $BINARY_NAME
    
    if (Test-Path $binaryPath) {
        Write-Info "Testing installation..."
        try {
            $null = & $binaryPath --version 2>$null
            Write-Success "Installation test passed!"
        }
        catch {
            Write-Warning "Binary installed but --version check failed"
        }
    }
    else {
        Write-Error "Binary not found at $binaryPath"
    }
}

# Main installation process
function Main {
    Write-Host "🚀 Ghost SSH Manager Installation Script" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    
    Write-Info "Install directory: $InstallDir"
    
    # Check if PowerShell execution policy allows script execution
    $executionPolicy = Get-ExecutionPolicy
    if ($executionPolicy -eq "Restricted") {
        Write-Warning "PowerShell execution policy is Restricted. You may need to run:"
        Write-Warning "Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser"
    }
    
    $version = Get-LatestVersion
    Install-Binary -Version $version
    Update-Path
    Test-Installation
    
    Write-Host ""
    Write-Success "Ghost SSH Manager installation complete!"
    Write-Host ""
    Write-Host "Run 'ghost' to get started (after restarting your terminal)"
    Write-Host "For help, visit: https://github.com/$REPO"
}

# Run installation
try {
    Main
}
catch {
    Write-Error "Installation failed: $_"
}

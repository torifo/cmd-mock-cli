# cmdock installer for Windows
# Usage: irm https://raw.githubusercontent.com/torifo/cmd-mock-cli/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'

$Repo = "torifo/cmd-mock-cli"
$BinName = "cmdock.exe"
$InstallDir = "$env:LOCALAPPDATA\Programs\cmdock"

# Fetch latest release tag
$ApiUrl = "https://api.github.com/repos/$Repo/releases/latest"
$Release = Invoke-RestMethod -Uri $ApiUrl -Headers @{ 'User-Agent' = 'cmdock-installer' }
$Tag = $Release.tag_name

if (-not $Tag) {
    Write-Error "Failed to fetch latest release tag"
    exit 1
}

$Archive = "cmdock-windows-x86_64.zip"
$Url = "https://github.com/$Repo/releases/download/$Tag/$Archive"

$TmpDir = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString()
New-Item -ItemType Directory -Path $TmpDir | Out-Null

try {
    Write-Host "Downloading cmdock $Tag (windows-x86_64)..."
    Invoke-WebRequest -Uri $Url -OutFile "$TmpDir\$Archive"
    Expand-Archive -Path "$TmpDir\$Archive" -DestinationPath $TmpDir

    if (-not (Test-Path "$TmpDir\$BinName")) {
        Write-Error "Binary not found in archive"
        exit 1
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item "$TmpDir\$BinName" "$InstallDir\$BinName" -Force

    Write-Host ""
    Write-Host "Installed: $InstallDir\$BinName"
    Write-Host ""

    # Check PATH
    $CurrentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($CurrentPath -notlike "*$InstallDir*") {
        Write-Host "NOTE: Add cmdock to your PATH by running:"
        Write-Host "  [Environment]::SetEnvironmentVariable('PATH', `"`$env:PATH;$InstallDir`", 'User')"
        Write-Host ""
    }

    Write-Host "Get started:"
    Write-Host "  cmdock"
} finally {
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}

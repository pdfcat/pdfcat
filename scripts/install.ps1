# pdfcat installer for Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/pdfcat/pdfcat/main/scripts/install.ps1 | iex

$ErrorActionPreference = "Stop"

# Configuration
$RepoUrl = "https://github.com/pdfcat/pdfcat"
$BinaryName = "pdfcat.exe"
$Version = if ($env:PDFCAT_VERSION) { $env:PDFCAT_VERSION } else { "latest" }

# NEW: Unified logging function
function Write-Log {
    param(
        [string]$Level,
        [string]$Message
    )

    $fc = $host.UI.RawUI.ForegroundColor
    $prefix = ""
    $color = $fc

    switch ($Level.ToUpper()) {
        'INFO' { $prefix = "    [INFO] "; $color = "Cyan" }
        'OK' { $prefix = "      [OK] "; $color = "Green" }
        'WARN' { $prefix = "    [WARN] "; $color = "Yellow" }
        'ERROR' { $prefix = "   [ERROR] "; $color = "Red" }
        'DEBUG' { $prefix = "   [DEBUG] "; $color = "DarkGray" }
        'HEADER' { $prefix = ""; $color = "Blue" } # For "Installing..."
        'BANNER' { $prefix = ""; $color = "Magenta" }
        'TITLE' { $prefix = ""; $color = "Cyan" }    # For section titles
        'SUCCESS' { $prefix = ""; $color = "Green" }   # For "Installation Complete!"
        'HELP' { $prefix = "   "; $color = $fc }     # For help text
        'EMPTY' { $prefix = ""; $color = $fc }       # For Write-Output ""
        default { $prefix = "        "; $color = $fc } # Default indentation for plain text
    }

    $host.UI.RawUI.ForegroundColor = $color
    
    # Handle multi-line messages
    $messageLines = $Message -split "`n"
    foreach ($line in $messageLines) {
        # Use Write-Host to print to console only and avoid polluting the output pipeline
        Write-Host ($prefix + $line)
    }
    
    $host.UI.RawUI.ForegroundColor = $fc
}

function Write-Banner {
    Write-Log -Level BANNER -Message @"
        _  __        _   
       | |/ _|      | |  
 _ __  __| | |_ ___ __ _| |_ 
| '_ \ / _` |  _/ __/ _` | __|
| |_) | (_| | || (_| (_| | |_ 
| .__/ \__,_|_| \___\__,_|\__|
| |                          
|_|  

    Concatenate PDF files into a single document
"@
    Write-Log -Level EMPTY -Message ""
}

function Test-DevMode {
    if (Test-Path "Cargo.toml") {
        # $content = Get-Content "Cargo.toml" -Raw
        return $true
    }
    return $false
}

function Get-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE
    
    switch ($arch) {
        "AMD64" { return "windows-x86_64" }
        "ARM64" { return "windows-aarch64" }
        default {
            Write-Log -Level ERROR -Message "Unsupported architecture: $arch"
            exit 1
        }
    }
}

function Test-Prerequisites {
    
    $missing = @()
    
    # Check for .NET (always available on Windows)
    # Check for cargo if in dev mode
    $devMode = Test-DevMode
    if ($devMode) {
        Write-Log -Level INFO -Message "Checking development dependencies"
    }

    if ($devMode -and -not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        $missing += "cargo (Rust toolchain)"
    }
    
    if ($missing.Count -gt 0) {
        Write-Log -Level ERROR -Message "Missing required tools:"
        $missing | ForEach-Object { Write-Log -Level DEFAULT -Message "  - $_" }
        Write-Log -Level EMPTY -Message ""
        Write-Log -Level WARN -Message "Please install the missing tools and try again."
        Write-Log -Level DEFAULT -Message "Rust toolchain: https://rustup.rs/"
        exit 1
    }
    
    if ($devMode) {
        Write-Log -Level OK -Message "All development prerequisites met"
    }
}

function Install-FromSource {
    Write-Log -Level HEADER -Message "🔨 Building from source..."
    
    if (-not (Test-Path "Cargo.toml")) {
        Write-Log -Level ERROR -Message "✗ Cargo.toml not found. Not in pdfcat directory?"
        exit 1
    }
    
    # Run tests
    Write-Log -Level TITLE -Message "Running tests..."
    try {
        cargo test --release
        Write-Log -Level OK -Message "✓ Tests passed"
    }
    catch {
        Write-Log -Level WARN -Message "⚠ Tests failed, but continuing with installation"
    }
    
    # Build
    Write-Log -Level TITLE -Message "Building release binary..."
    cargo build --release
    
    $script:BinaryPath = "target\release\$BinaryName"
    
    if (-not (Test-Path $script:BinaryPath)) {
        Write-Log -Level ERROR -Message "✗ Build failed - binary not found"
        exit 1
    }
    
    Write-Log -Level OK -Message "✓ Build successful"
}

function Get-Binary {
    <#
.SYNOPSIS
Download the latest pdfcat release for the current OS/arch.

.DESCRIPTION
This function fetches the latest release asset for the current operating system and architecture
from a GitHub repository. It handles both .zip (Windows) and .tar.gz (Linux/macOS) archives.
It attempts to use a GitHub token if available for higher rate limits but works without
authentication for public repositories.

.PARAMETER Owner
The owner of the GitHub repository (e.g., 'pdfcat').

.PARAMETER Repo
The name of the GitHub repository (e.g., 'pdfcat').

.PARAMETER OutputDir
The local directory where the extracted binary should be placed. Defaults to the current working directory.

.EXAMPLE
Get-Binary
# Downloads the latest pdfcat binary to the current directory.

.EXAMPLE
Get-Binary -OutputDir 'C:\Tools' -Owner 'mycompany' -Repo 'tool-name'
# Downloads the latest 'tool-name' binary from 'mycompany' to C:\Tools.
#>
    param(
        [string]$Owner = "pdfcat",
        [string]$Repo = "pdfcat",
        [string]$OutputDir = "$PWD"
    )

    Write-Log -Level HEADER -Message "📥 Downloading artificats from $Owner/$Repo (latest release)..."

    # --- 1. Determine OS and architecture ---
    $OS = if ($IsWindows) { "windows" } elseif ($IsLinux) { "linux" } elseif ($IsMacOS) { "darwin" } else {
        Write-Log -Level ERROR -Message "Unsupported OS detected."
        exit 1
    }

    $Arch = if ([System.Environment]::Is64BitOperatingSystem) {
        # Check for AMD64 (common x64 term) or fallback to aarch64 if not.
        if ($env:PROCESSOR_ARCHITECTURE -eq "AMD64" -or $env:PROCESSOR_ARCHITEW6432 -eq "AMD64") { "x86_64" } else { "aarch64" }
    }
    else {
        Write-Log -Level ERROR -Message "32-bit OS not supported."
        exit 1
    }

    # Write-Log -Level INFO -Message "Platform detected: $OS/$Arch"

    # --- 2. Determine Asset Pattern and Binary Name ---
    $ext = if ($OS -eq "windows") { "zip" } else { "tar.gz" }
    $assetPattern = "$Repo*-$OS-$Arch.$ext"
    $BinaryName = if ($OS -eq "windows") { "$Repo.exe" } else { $Repo }

    # --- 3. Token Setup (for authenticated API call) ---
    $token = $env:GITHUB_TOKEN
    if (-not $token) {
        try {
            $token = gh auth token 2>$null
        }
        catch {
            Write-Log -Level WARN -Message "No GITHUB_TOKEN environment variable found and 'gh auth token' failed. Will try unauthenticated request."
        }
    }

    # --- 4. Headers for Release API call (JSON response) ---
    $apiHeaders = @{
        "User-Agent"           = "PowerShell"
        "Accept"               = "application/vnd.github+json"
        "X-GitHub-Api-Version" = "2022-11-28"
    }
    if ($token) {
        $apiHeaders["Authorization"] = "token $token"
        Write-Log -Level INFO -Message "Using GitHub token for authenticated release lookup."
    }
    else {
        Write-Log -Level INFO -Message "No token available, using unauthenticated release lookup."
    }

    # --- 5. Fetch latest release ---
    $releaseUrl = "https://api.github.com/repos/$Owner/$Repo/releases/latest"
    try {
        $release = Invoke-RestMethod -Uri $releaseUrl -Headers $apiHeaders
    }
    catch {
        Write-Log -Level ERROR -Message "Failed to fetch latest release info from ${releaseUrl}: $($_.Exception.Message)"
        exit 1
    }

    $tag = $release.tag_name
    Write-Log -Level INFO -Message "Latest release found: $tag"

    # --- 6. Find the correct asset ---
    $asset = $release.assets | Where-Object { $_.name -like $assetPattern } | Select-Object -First 1
    if (-not $asset) {
        Write-Log -Level ERROR -Message "No matching asset found for pattern '$assetPattern' in release $tag."
        exit 1
    }

    $assetUrl = $asset.url # API URL for download
    Write-Log -Level INFO -Message "Downloading archive $($asset.name)"

    # --- 7. Setup temporary directory for download and extraction ---
    $tmpDir = New-TemporaryFile | ForEach-Object { Remove-Item $_ -Force; New-Item -ItemType Directory -Path $_ }
    $archivePath = Join-Path $tmpDir $asset.name

    # --- 8. Headers for Binary Download (Octet-Stream response) ---
    $downloadHeaders = @{
        "User-Agent" = "PowerShell"
        "Accept"     = "application/octet-stream"
    }
    # Token must be included here for private assets
    if ($token) {
        $downloadHeaders["Authorization"] = "token $token"
    }

    try {
        Invoke-WebRequest -Uri $assetUrl -Headers $downloadHeaders -OutFile $archivePath
        Write-Log -Level OK -Message "Archive downloaded to $archivePath"
    }
    catch {
        Write-Log -Level ERROR -Message "Download failed: $($_.Exception.Message)"
        Remove-Item $tmpDir -Recurse -Force
        exit 1
    }

    # --- 9. Extraction ---
    try {
        if ($ext -eq "zip") {
            Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force
        }
        else {
            # Use 'tar' utility for .tar.gz, which is available on Windows, Linux, and macOS
            tar -xf $archivePath -C $tmpDir
        }
    }
    catch {
        Write-Log -Level ERROR -Message "Extraction failed: $($_.Exception.Message)"
        Remove-Item $tmpDir -Recurse -Force
        exit 1
    }

    # --- 10. Locate binary and move to final output directory ---

    # Search for the binary recursively in case it was extracted into a sub-folder (common for archives)
    $finalBinary = Get-ChildItem -Path $tmpDir -Filter $BinaryName -Recurse | Select-Object -First 1

    if (-not $finalBinary) {
        Write-Log -Level ERROR -Message "✗ Binary '$BinaryName' not found in the extracted archive. Check release structure."
        Remove-Item $tmpDir -Recurse -Force
        exit 1
    }

    # Ensure OutputDir exists
    $finalOutputPath = Join-Path $OutputDir $BinaryName
    if (-not (Test-Path $OutputDir -PathType Container)) {
        Write-Log -Level DEBUG -Message "Creating output directory: $OutputDir"
        New-Item -ItemType Directory -Path $OutputDir | Out-Null
    }

    Move-Item -Path $finalBinary.FullName -Destination $finalOutputPath -Force

    # Clean up temp directory
    Remove-Item $tmpDir -Recurse -Force | Out-Null

    Write-Log -Level OK -Message "Binary unpacked to: $finalOutputPath"

    return $finalOutputPath
}

function Get-InstallDir {
    if ($env:PDFCAT_INSTALL_DIR) {
        return $env:PDFCAT_INSTALL_DIR
    }
    
    $devMode = Test-DevMode
    if ($devMode) {
        return Join-Path (Get-Location) "target\release"
    }
    
    # Check for standard Windows locations
    $localAppData = $env:LOCALAPPDATA
    $installDir = Join-Path $localAppData "Programs\pdfcat"
    
    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }
    
    return $installDir
}

function Install-Binary {
    Write-Log -Level HEADER -Message "📋 Copying artificats"
    
    $installDir = Get-InstallDir
    Write-Log -Level INFO -Message "Installation directory: $installDir"
    
    $installPath = Join-Path $installDir $BinaryName
    
    $devMode = Test-DevMode
    if ($devMode -and (Test-Path $script:BinaryPath)) {
        Write-Log -Level DEBUG -Message "Binary built at: $script:BinaryPath"
        $script:InstalledPath = $script:BinaryPath
    }
    else {
        Copy-Item $script:BinaryPath $installPath -Force
        $script:InstalledPath = $installPath
    }
    
    Write-Log -Level OK -Message "Installed pdfcat to: $installPath"
}

function Test-InPath {
    $devMode = Test-DevMode
    if ($devMode) {
        return;
    }
    
    $installDir = Get-InstallDir
    $pathDirs = $env:Path -split ';'
    
    if ($pathDirs -contains $installDir) {
        Write-Log -Level OK -Message "Installation directory is in PATH"
        # return $true
    }
    else {
        Write-Log -Level WARN -Message "pdfcat installation directory is not in PATH"
        # Write-Log -Level EMPTY -Message ""
        # Write-Log -Level INFO -Message "Adding pdfcat directory to PATH (current user)"
        
        # Add to user PATH
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $newPath = "$userPath;$installDir"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        
        # Update current session
        $env:Path = "$env:Path;$installDir"
        
        Write-Log -Level OK -Message "Added pdfcat directory to PATH"
        Write-Log -Level WARN -Message "  Note: Restart your terminal for PATH changes to take effect"
        # return $false
    }
}

function Test-Installation {
    Write-Log -Level HEADER -Message "🔍 Verifying installation"
    
    if (Test-Path $script:InstalledPath) {
        Write-Log -Level OK -Message "✓ Binary exists"
        
        try {
            $version = & $script:InstalledPath --version
            Write-Log -Level OK -Message "✓ $version"
        }
        catch {
            Write-Log -Level WARN -Message "⚠ Binary installed but version check failed"
        }
    }
    else {
        Write-Log -Level ERROR -Message "✗ Binary not found at expected location"
        exit 1
    }
}

function Write-Success {
    Write-Log -Level EMPTY -Message ""
    Write-Log -Level SUCCESS -Message "✓ Successfully installed pdfcat"
    Write-Log -Level EMPTY -Message ""
    
    $devMode = Test-DevMode
    if ($devMode) {
        Write-Log -Level TITLE -Message "Development mode:"
        Write-Log -Level HELP -Message "Run: cargo run -- --help"
        Write-Log -Level HELP -Message "Or:  .\target\release\pdfcat.exe --help"
    }
    else {
        Write-Log -Level TITLE -Message "Get started:"
        Write-Log -Level HELP -Message "pdfcat --help       Show help"
        Write-Log -Level HELP -Message "pdfcat --version    Show version"
        Write-Log -Level EMPTY -Message ""
        Write-Log -Level TITLE -Message "Example usage:"
        Write-Log -Level HELP -Message "pdfcat file1.pdf file2.pdf -o merged.pdf"
        Write-Log -Level HELP -Message "pdfcat *.pdf -o combined.pdf --bookmarks"
    }
    
    Write-Log -Level EMPTY -Message ""
    Write-Log -Level TITLE -Message "Documentation: $RepoUrl"
    Write-Log -Level TITLE -Message "Report issues: $RepoUrl/issues"
    Write-Log -Level EMPTY -Message ""
}

function Write-Error {
    Write-Log -Level EMPTY -Message ""
    Write-Log -Level ERROR -Message "═══════════════════════════════════════"
    Write-Log -Level ERROR -Message "  ✗ Installation Failed"
    Write-Log -Level ERROR -Message "═══════════════════════════════════════"
    Write-Log -Level EMPTY -Message ""
    Write-Log -Level WARN -Message "For help:"
    Write-Log -Level DEFAULT -Message "  • Check the documentation: $RepoUrl"
    Write-Log -Level DEFAULT -Message "  • Report an issue: $RepoUrl/issues"
    Write-Log -Level DEFAULT -Message "  • Manual installation: $RepoUrl#installation"
    Write-Log -Level EMPTY -Message ""
}

# Main installation flow
function Main {
    try {
        # Write-Banner
        $titleMarkdown = "**pdfcat CLI**"
        $title = ($titleMarkdown | ConvertFrom-Markdown -AsVT100EncodedString).VT100EncodedString;
        Write-Log -Level BANNER -Message "Installing $title"        
        $platform = Get-Platform
        Write-Log -Level HELP -Message "You are about to install pdfcat CLI on $platform platform"        
        
        $devMode = Test-DevMode
        if ($devMode) {
            Write-Log -Level INFO -Message "Development mode detected"
        }
        
        Test-Prerequisites
        Write-Log -Level EMPTY -Message ""
        
        $downloadedBinaryPath = $null # Keep track of file downloaded to $PWD

        if ($devMode) {
            Install-FromSource
        }
        else {
            if ($env:PDFCAT_BUILD_FROM_SOURCE -eq "true" -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
                Write-Log -Level TITLE -Message "Building from source (requested via PDFCAT_BUILD_FROM_SOURCE)"
                
                if (-not (Test-Path "Cargo.toml")) {
                    git clone $RepoUrl pdfcat
                    Set-Location pdfcat
                }
                
                Install-FromSource
            }
            else {
                # Get-Binary returns the path to the file (in $PWD)
                $downloadedBinaryPath = Get-Binary
                # Set $script:BinaryPath so Install-Binary can find it
                $script:BinaryPath = $downloadedBinaryPath
            }
        }
        
        Write-Log -Level EMPTY -Message ""
        Install-Binary

        # Cleanup the binary downloaded to $PWD
        if ($downloadedBinaryPath -and (Test-Path $downloadedBinaryPath)) {
            Write-Log -Level INFO -Message "Removing temporary artificats"
            Remove-Item -Force $downloadedBinaryPath
        }
        
        Test-InPath
        Write-Log -Level EMPTY -Message ""
        
        Test-Installation
        Write-Success

        
    }
    catch {
        Write-Error
        Write-Log -Level ERROR -Message "Details: $_"
        exit 1
    }
}

# Run main
Main
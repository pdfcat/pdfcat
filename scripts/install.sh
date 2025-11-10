#!/usr/bin/env bash
# pdfcat installer - Universal installation script for Linux/macOS/Windows (Git Bash/WSL)
# Usage: curl -sSf https://raw.githubusercontent.com/pdfcat/pdfcat/main/scripts/install.sh | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

# Configuration
REPO_URL="https://github.com/pdfcat/pdfcat"
BINARY_NAME="pdfcat"
VERSION="${PDFCAT_VERSION:-latest}"

# Detect OS and architecture
detect_platform() {
    local os arch
    
    os="$(uname -s)"
    arch="$(uname -m)"
    
    case "$os" in
        Linux)
            OS="linux"
            ;;
        Darwin)
            OS="macos"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            OS="windows"
            ;;
        *)
            echo -e "${RED}✗ Unsupported operating system: $os${RESET}"
            exit 1
            ;;
    esac
    
    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            echo -e "${RED}✗ Unsupported architecture: $arch${RESET}"
            exit 1
            ;;
    esac
    
    PLATFORM="${OS}-${ARCH}"
}

# Check if running in development mode
check_dev_mode() {
    if [ -f "Cargo.toml" ] && grep -q "name = \"pdfcat\"" Cargo.toml 2>/dev/null; then
        DEV_MODE=true
        echo -e "${CYAN}📦 Development mode detected${RESET}"
    else
        DEV_MODE=false
    fi
}

# Print banner
print_banner() {
    echo -e "${BOLD}${MAGENTA}"
    cat << "EOF"

   pdfcat                                   
   Concatenate PDF files into a single document.
EOF
    echo -e "${RESET}"
}

# Check prerequisites
check_prerequisites() {
    echo -e "${BLUE}🔍 Checking prerequisites...${RESET}"
    
    local missing=()
    
    # Check for curl or wget
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        missing+=("curl or wget")
    fi
    
    # Check for tar (for binary releases)
    if ! command -v tar >/dev/null 2>&1 && [ "$DEV_MODE" = false ]; then
        missing+=("tar")
    fi
    
    # Check for cargo (for dev mode or source install)
    if ! command -v cargo >/dev/null 2>&1 && [ "$DEV_MODE" = true ]; then
        missing+=("cargo (Rust toolchain)")
    fi
    
    if [ ${#missing[@]} -gt 0 ]; then
        echo -e "${RED}✗ Missing required tools:${RESET}"
        printf '  - %s\n' "${missing[@]}"
        echo ""
        echo -e "${YELLOW}Please install the missing tools and try again.${RESET}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ All prerequisites met${RESET}"
}

# Install from source (development mode)
install_from_source() {
    echo -e "${BLUE}🔨 Building from source...${RESET}"
    
    if [ ! -f "Cargo.toml" ]; then
        echo -e "${RED}✗ Cargo.toml not found. Not in pdfcat directory?${RESET}"
        exit 1
    fi
    
    # Run tests first
    echo -e "${CYAN}Running tests...${RESET}"
    if cargo test --release; then
        echo -e "${GREEN}✓ Tests passed${RESET}"
    else
        echo -e "${YELLOW}⚠ Tests failed, but continuing with installation${RESET}"
    fi
    
    # Build release binary
    echo -e "${CYAN}Building release binary...${RESET}"
    cargo build --release
    
    BINARY_PATH="target/release/$BINARY_NAME"
    if [ "$OS" = "windows" ]; then
        BINARY_PATH="${BINARY_PATH}.exe"
    fi
    
    if [ ! -f "$BINARY_PATH" ]; then
        echo -e "${RED}✗ Build failed - binary not found${RESET}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Build successful${RESET}"
}

# Download pre-built binary
download_binary() {
    echo -e "${BLUE}📥 Downloading pdfcat for $PLATFORM...${RESET}"
    
    local download_url
    local archive_name="pdfcat-${VERSION}-${PLATFORM}.tar.gz"
    
    if [ "$VERSION" = "latest" ]; then
        download_url="${REPO_URL}/releases/latest/download/${archive_name}"
    else
        download_url="${REPO_URL}/releases/download/${VERSION}/${archive_name}"
    fi
    
    local tmp_dir
    tmp_dir=$(mktemp -d)
    cd "$tmp_dir"
    
    echo -e "${CYAN}Downloading from: $download_url${RESET}"
    
    if command -v curl >/dev/null 2>&1; then
        curl -L -o "$archive_name" "$download_url" || {
            echo -e "${RED}✗ Download failed${RESET}"
            exit 1
        }
    else
        wget -O "$archive_name" "$download_url" || {
            echo -e "${RED}✗ Download failed${RESET}"
            exit 1
        }
    fi
    
    echo -e "${CYAN}Extracting...${RESET}"
    tar -xzf "$archive_name"
    
    BINARY_PATH="$tmp_dir/$BINARY_NAME"
    if [ "$OS" = "windows" ]; then
        BINARY_PATH="${BINARY_PATH}.exe"
    fi
    
    if [ ! -f "$BINARY_PATH" ]; then
        echo -e "${RED}✗ Binary not found in archive${RESET}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Download successful${RESET}"
}

# Determine installation directory
get_install_dir() {
    if [ -n "$PDFCAT_INSTALL_DIR" ]; then
        INSTALL_DIR="$PDFCAT_INSTALL_DIR"
    elif [ "$DEV_MODE" = true ]; then
        # For dev mode, install to project directory
        INSTALL_DIR="$(pwd)/target/release"
    elif [ -w "/usr/local/bin" ]; then
        INSTALL_DIR="/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
    else
        INSTALL_DIR="$HOME/bin"
        mkdir -p "$INSTALL_DIR"
    fi
    
    echo -e "${CYAN}Installation directory: $INSTALL_DIR${RESET}"
}

# Install binary
install_binary() {
    echo -e "${BLUE}📦 Installing pdfcat...${RESET}"
    
    get_install_dir
    
    local install_path="$INSTALL_DIR/$BINARY_NAME"
    if [ "$OS" = "windows" ]; then
        install_path="${install_path}.exe"
    fi
    
    # Copy binary
    if [ "$DEV_MODE" = true ] && [ -f "$BINARY_PATH" ]; then
        # In dev mode, binary is already in target/release
        echo -e "${CYAN}Binary built at: $BINARY_PATH${RESET}"
    else
        cp "$BINARY_PATH" "$install_path"
        chmod +x "$install_path"
    fi
    
    echo -e "${GREEN}✓ Installed to: $install_path${RESET}"
    
    INSTALLED_PATH="$install_path"
}

# Check if installation directory is in PATH
check_path() {
    if [ "$DEV_MODE" = true ]; then
        return 0  # Skip path check in dev mode
    fi
    
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo -e "${GREEN}✓ Installation directory is in PATH${RESET}"
        return 0
    else
        echo -e "${YELLOW}⚠ Installation directory is not in PATH${RESET}"
        echo ""
        echo -e "${CYAN}Add the following to your shell configuration:${RESET}"
        
        case "$SHELL" in
            */bash)
                echo -e "${BOLD}  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc${RESET}"
                echo -e "${BOLD}  source ~/.bashrc${RESET}"
                ;;
            */zsh)
                echo -e "${BOLD}  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc${RESET}"
                echo -e "${BOLD}  source ~/.zshrc${RESET}"
                ;;
            */fish)
                echo -e "${BOLD}  set -U fish_user_paths $INSTALL_DIR \$fish_user_paths${RESET}"
                ;;
            *)
                echo -e "${BOLD}  export PATH=\"$INSTALL_DIR:\$PATH\"${RESET}"
                ;;
        esac
        echo ""
        return 1
    fi
}

# Verify installation
verify_installation() {
    echo -e "${BLUE}🔍 Verifying installation...${RESET}"
    
    if [ -x "$INSTALLED_PATH" ]; then
        echo -e "${GREEN}✓ Binary is executable${RESET}"
        
        # Try to run version command
        if "$INSTALLED_PATH" --version >/dev/null 2>&1; then
            local version
            version=$("$INSTALLED_PATH" --version)
            echo -e "${GREEN}✓ $version${RESET}"
        else
            echo -e "${YELLOW}⚠ Binary installed but version check failed${RESET}"
        fi
    else
        echo -e "${RED}✗ Binary is not executable${RESET}"
        exit 1
    fi
}

# Print success message
print_success() {
    echo ""
    echo -e "${BOLD}${GREEN}═══════════════════════════════════════${RESET}"
    echo -e "${BOLD}${GREEN}  ✓ Installation Complete!${RESET}"
    echo -e "${BOLD}${GREEN}═══════════════════════════════════════${RESET}"
    echo ""
    
    if [ "$DEV_MODE" = true ]; then
        echo -e "${CYAN}Development mode:${RESET}"
        echo -e "  Run: ${BOLD}cargo run -- --help${RESET}"
        echo -e "  Or:  ${BOLD}./target/release/pdfcat --help${RESET}"
    else
        echo -e "${CYAN}Get started:${RESET}"
        echo -e "  ${BOLD}pdfcat --help${RESET}          Show help"
        echo -e "  ${BOLD}pdfcat --version${RESET}       Show version"
        echo ""
        echo -e "${CYAN}Example usage:${RESET}"
        echo -e "  ${BOLD}pdfcat file1.pdf file2.pdf -o merged.pdf${RESET}"
        echo -e "  ${BOLD}pdfcat *.pdf -o combined.pdf --bookmarks${RESET}"
    fi
    
    echo ""
    echo -e "${CYAN}Documentation:${RESET} ${BLUE}${REPO_URL}${RESET}"
    echo -e "${CYAN}Report issues:${RESET} ${BLUE}${REPO_URL}/issues${RESET}"
    echo ""
}

# Handle installation errors
handle_error() {
    echo ""
    echo -e "${RED}═══════════════════════════════════════${RESET}"
    echo -e "${RED}  ✗ Installation Failed${RESET}"
    echo -e "${RED}═══════════════════════════════════════${RESET}"
    echo ""
    echo -e "${YELLOW}For help:${RESET}"
    echo -e "  • Check the documentation: ${BLUE}${REPO_URL}${RESET}"
    echo -e "  • Report an issue: ${BLUE}${REPO_URL}/issues${RESET}"
    echo -e "  • Manual installation: ${BLUE}${REPO_URL}#installation${RESET}"
    echo ""
}

# Cleanup on exit
cleanup() {
    if [ -n "$tmp_dir" ] && [ -d "$tmp_dir" ]; then
        rm -rf "$tmp_dir"
    fi
}

# Main installation flow
main() {
    trap cleanup EXIT
    trap handle_error ERR
    
    print_banner
    
    echo -e "${BOLD}Installing pdfcat - PDF Concatenation Tool${RESET}"
    echo ""
    
    detect_platform
    echo -e "${CYAN}Platform: $PLATFORM${RESET}"
    
    check_dev_mode
    check_prerequisites
    echo ""
    
    if [ "$DEV_MODE" = true ]; then
        install_from_source
    else
        # Check if user wants to build from source
        if [ "${PDFCAT_BUILD_FROM_SOURCE:-false}" = "true" ] && command -v cargo >/dev/null 2>&1; then
            echo -e "${CYAN}Building from source (requested via PDFCAT_BUILD_FROM_SOURCE)${RESET}"
            
            # Clone repository if needed
            if [ ! -f "Cargo.toml" ]; then
                git clone "$REPO_URL" pdfcat
                cd pdfcat
            fi
            
            install_from_source
        else
            download_binary
        fi
    fi
    
    echo ""
    install_binary
    echo ""
    
    check_path
    echo ""
    
    verify_installation
    print_success
}

# Run main function
main "$@"
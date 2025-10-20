#!/bin/bash

# Ghost SSH Manager Installation Script
# For Linux and macOS

set -e

# Configuration
REPO="smoke516/ghost"
BINARY_NAME="ghost"
INSTALL_DIR="$HOME/.local/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
log() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case $os in
        linux*)
            OS="linux"
            ;;
        darwin*)
            OS="macos"
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac
    
    case $arch in
        x86_64|amd64)
            ARCH="x64"
            ;;
        arm64|aarch64)
            ARCH="arm64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            ;;
    esac
    
    PLATFORM="${OS}-${ARCH}"
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Get the latest release version
get_latest_version() {
    log "Fetching latest release information..."
    
    if command_exists curl; then
        VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    elif command_exists wget; then
        VERSION=$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
    
    if [ -z "$VERSION" ]; then
        error "Could not determine latest version"
    fi
    
    log "Latest version: $VERSION"
}

# Download and install the binary
install_binary() {
    local download_url="https://github.com/$REPO/releases/download/$VERSION/ghost-${PLATFORM}.tar.gz"
    local temp_dir=$(mktemp -d)
    local archive_path="$temp_dir/ghost.tar.gz"
    
    log "Downloading from: $download_url"
    
    if command_exists curl; then
        curl -L -o "$archive_path" "$download_url"
    elif command_exists wget; then
        wget -O "$archive_path" "$download_url"
    else
        error "Neither curl nor wget found"
    fi
    
    if [ ! -f "$archive_path" ]; then
        error "Download failed"
    fi
    
    log "Extracting archive..."
    tar -xzf "$archive_path" -C "$temp_dir"
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Move binary to install directory
    mv "$temp_dir/$BINARY_NAME" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    
    # Clean up
    rm -rf "$temp_dir"
    
    success "Ghost SSH Manager installed to $INSTALL_DIR/$BINARY_NAME"
}

# Check if install directory is in PATH
check_path() {
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warn "Install directory $INSTALL_DIR is not in your PATH"
        echo
        echo "Add the following line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        echo
        echo "Then reload your shell or run:"
        echo "  source ~/.bashrc  # or ~/.zshrc"
    fi
}

# Test installation
test_installation() {
    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        log "Testing installation..."
        if "$INSTALL_DIR/$BINARY_NAME" --version >/dev/null 2>&1; then
            success "Installation test passed!"
        else
            warn "Binary installed but --version check failed"
        fi
    else
        error "Binary not found at $INSTALL_DIR/$BINARY_NAME"
    fi
}

# Main installation process
main() {
    echo "🚀 Ghost SSH Manager Installation Script"
    echo "========================================"
    echo
    
    detect_platform
    log "Detected platform: $PLATFORM"
    
    get_latest_version
    install_binary
    check_path
    test_installation
    
    echo
    success "Ghost SSH Manager installation complete!"
    echo
    echo "Run 'ghost' to get started (after adding to PATH if needed)"
    echo "For help, visit: https://github.com/$REPO"
}

# Run installation
main "$@"

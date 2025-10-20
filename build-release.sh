#!/bin/bash

# Ghost SSH Manager Release Build Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Configuration
VERSION=${1:-$(grep '^version' Cargo.toml | sed 's/.*= "\([^"]*\)".*/\1/')}
DIST_DIR="./dist"

log "Building Ghost SSH Manager v${VERSION}"
log "Output directory: ${DIST_DIR}"

# Clean and create dist directory
rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}"

# Build Linux x64
log "Building for Linux x64..."
cargo build --release --target x86_64-unknown-linux-gnu
if [ $? -eq 0 ]; then
    cp target/x86_64-unknown-linux-gnu/release/ghost "${DIST_DIR}/ghost-linux-x64"
    
    # Create tar.gz archive
    cd "${DIST_DIR}"
    tar -czf ghost-linux-x64.tar.gz ghost-linux-x64
    # Also create a symlink for backward compatibility
    ln -sf ghost-linux-x64 ghost
    cd ..
    
    success "Linux x64 build completed"
else
    error "Linux x64 build failed"
fi

# Build Windows x64 (if cross-compilation is available)
log "Building for Windows x64..."
if rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    cargo build --release --target x86_64-pc-windows-gnu
    if [ $? -eq 0 ]; then
        cp target/x86_64-pc-windows-gnu/release/ghost.exe "${DIST_DIR}/ghost-windows-x64.exe"
        
        # Create zip archive
        cd "${DIST_DIR}"
        zip ghost-windows-x64.zip ghost-windows-x64.exe
        # Also create a symlink for backward compatibility
        ln -sf ghost-windows-x64.exe ghost.exe
        cd ..
        
        success "Windows x64 build completed"
    else
        warn "Windows x64 build failed, but continuing..."
    fi
else
    warn "Windows target not installed. Run: rustup target add x86_64-pc-windows-gnu"
    warn "You may also need to install mingw-w64: sudo apt install mingw-w64"
fi

# Build Linux ARM64 (if available)
log "Building for Linux ARM64..."
if rustup target list --installed | grep -q "aarch64-unknown-linux-gnu"; then
    if which aarch64-linux-gnu-gcc >/dev/null 2>&1; then
        export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
        export CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
        cargo build --release --target aarch64-unknown-linux-gnu
        if [ $? -eq 0 ]; then
            cp target/aarch64-unknown-linux-gnu/release/ghost "${DIST_DIR}/ghost-linux-arm64"
            
            # Create tar.gz archive
            cd "${DIST_DIR}"
            tar -czf ghost-linux-arm64.tar.gz ghost-linux-arm64
            cd ..
            
            success "Linux ARM64 build completed"
        else
            warn "Linux ARM64 build failed, but continuing..."
        fi
    else
        warn "ARM64 cross-compiler not found. Install with: sudo apt install gcc-aarch64-linux-gnu"
    fi
else
    warn "Linux ARM64 target not installed. Run: rustup target add aarch64-unknown-linux-gnu"
fi

# Try to build macOS (this will likely fail without proper toolchain)
log "Attempting macOS builds..."
for target in "x86_64-apple-darwin" "aarch64-apple-darwin"; do
    if rustup target list --installed | grep -q "$target"; then
        log "Building for $target..."
        cargo build --release --target "$target" 2>/dev/null
        if [ $? -eq 0 ]; then
            arch=$(echo $target | cut -d'-' -f1)
            if [ "$arch" = "x86_64" ]; then
                arch="x64"
            elif [ "$arch" = "aarch64" ]; then
                arch="arm64"
            fi
            
            cp "target/$target/release/ghost" "${DIST_DIR}/ghost-macos-${arch}"
            
            # Create tar.gz archive
            cd "${DIST_DIR}"
            tar -czf "ghost-macos-${arch}.tar.gz" "ghost-macos-${arch}"
            cd ..
            
            success "macOS $arch build completed"
        else
            warn "macOS $target build failed (cross-compilation toolchain likely not available)"
        fi
    else
        warn "macOS $target not installed"
    fi
done

# Show what was built
log "Build summary:"
ls -lh "${DIST_DIR}/"

# Create checksums
log "Creating checksums..."
cd "${DIST_DIR}"
for file in *.tar.gz *.zip; do
    if [ -f "$file" ]; then
        sha256sum "$file" >> checksums.txt
    fi
done

if [ -f checksums.txt ]; then
    success "Checksums created:"
    cat checksums.txt
fi

cd ..

success "Build process completed!"
log "Binaries are in: ${DIST_DIR}/"
log "Next steps:"
log "  1. Test the binaries on their respective platforms"
log "  2. Create a GitHub release"
log "  3. Upload the binary archives as release assets"
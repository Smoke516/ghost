#!/bin/bash

# GitHub Release Creation Script for Ghost SSH Manager

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
REPO="Smoke516/ghost"
VERSION=${1:-$(grep '^version' Cargo.toml | sed 's/.*= "\([^"]*\)".*/\1/')}
TAG="v${VERSION}"
DIST_DIR="./dist"

log "Creating GitHub release for Ghost SSH Manager v${VERSION}"

# Check if gh CLI is installed
if ! command -v gh >/dev/null 2>&1; then
    error "GitHub CLI (gh) is not installed. Please install it first: https://cli.github.com/"
fi

# Check if user is authenticated
if ! gh auth status >/dev/null 2>&1; then
    error "You're not authenticated with GitHub CLI. Run: gh auth login"
fi

# Check if dist directory exists and has files
if [ ! -d "$DIST_DIR" ]; then
    error "Distribution directory $DIST_DIR not found. Run ./build-release.sh first."
fi

if [ ! -f "$DIST_DIR/ghost-linux-x64.tar.gz" ] || [ ! -f "$DIST_DIR/ghost-windows-x64.zip" ]; then
    error "Required binary archives not found in $DIST_DIR. Run ./build-release.sh first."
fi

# Create release notes
RELEASE_NOTES=$(cat << EOF
# Ghost SSH Manager ${TAG}

A powerful SSH connection manager and terminal multiplexer built in Rust.

## Features
- Fast SSH connection management
- Terminal multiplexing capabilities
- Cross-platform support (Linux, Windows)
- Easy installation and configuration

## Installation

### Linux (x64)
\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash
\`\`\`

### Windows (x64)
\`\`\`powershell
irm https://raw.githubusercontent.com/${REPO}/main/install.ps1 | iex
\`\`\`

### Manual Installation
Download the appropriate binary for your platform from the assets below, extract it, and place it in your PATH.

## Checksums
\`\`\`
$(cat $DIST_DIR/checksums.txt 2>/dev/null || echo "No checksums available")
\`\`\`

## What's Changed
- Initial release of Ghost SSH Manager
- Cross-platform support for Linux and Windows
- Automated installation scripts

**Full Changelog**: https://github.com/${REPO}/commits/${TAG}
EOF
)

log "Release notes:"
echo "$RELEASE_NOTES"
echo

# Ask for confirmation
read -p "Do you want to create this release? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    log "Release creation cancelled."
    exit 0
fi

# Check if tag already exists
if git tag -l | grep -q "^${TAG}$"; then
    warn "Tag $TAG already exists locally. Deleting it..."
    git tag -d "$TAG"
fi

# Check if tag exists on remote
if git ls-remote --tags origin | grep -q "refs/tags/${TAG}$"; then
    warn "Tag $TAG already exists on remote. This will update the release."
fi

# Create and push tag
log "Creating and pushing tag $TAG..."
git tag -a "$TAG" -m "Release $TAG"
git push origin "$TAG" || warn "Tag push failed, continuing..."

# Create the release
log "Creating GitHub release..."
gh release create "$TAG" \
    --title "Ghost SSH Manager $TAG" \
    --notes "$RELEASE_NOTES" \
    --latest

# Upload assets
log "Uploading binary assets..."
cd "$DIST_DIR"
for file in *.tar.gz *.zip checksums.txt; do
    if [ -f "$file" ]; then
        log "Uploading $file..."
        gh release upload "$TAG" "$file" --clobber
    fi
done

cd ..

success "GitHub release created successfully!"
log "Release URL: https://github.com/$REPO/releases/tag/$TAG"
log ""
log "Next steps:"
log "1. Test the installation scripts:"
log "   curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | bash"
log "2. Update your README.md with installation instructions"
log "3. Share the release with users!"
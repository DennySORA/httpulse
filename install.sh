#!/bin/bash
set -euo pipefail

REPO="DennySORA/httpulse"
BINARY_NAME="httpulse"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; exit 1; }

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        *)       error "Unsupported OS: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Get target triple
get_target() {
    local os="$1"
    local arch="$2"

    case "${os}-${arch}" in
        linux-x86_64)   echo "x86_64-unknown-linux-musl" ;;
        linux-aarch64)  echo "aarch64-unknown-linux-musl" ;;
        darwin-x86_64)  echo "x86_64-apple-darwin" ;;
        darwin-aarch64) echo "aarch64-apple-darwin" ;;
        *)              error "Unsupported platform: ${os}-${arch}" ;;
    esac
}

# Get latest release version
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"

    if command -v curl &> /dev/null; then
        curl -fsSL "$url" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget &> /dev/null; then
        wget -qO- "$url" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "curl or wget is required"
    fi
}

# Download file
download() {
    local url="$1"
    local output="$2"

    info "Downloading from: $url"

    if command -v curl &> /dev/null; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget &> /dev/null; then
        wget -q "$url" -O "$output"
    else
        error "curl or wget is required"
    fi
}

main() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}       ${GREEN}httpulse${NC} Installer              ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
    echo ""

    # Detect platform
    local os=$(detect_os)
    local arch=$(detect_arch)
    local target=$(get_target "$os" "$arch")

    info "Detected platform: ${os}/${arch}"
    info "Target: ${target}"

    # Get version
    local version="${VERSION:-}"
    if [[ -z "$version" ]]; then
        info "Fetching latest version..."
        version=$(get_latest_version)
    fi

    if [[ -z "$version" ]]; then
        error "Failed to determine version. Set VERSION env var or check network."
    fi

    info "Version: ${version}"

    # Build download URL
    local filename="${BINARY_NAME}-${target}.tar.gz"
    local url="https://github.com/${REPO}/releases/download/${version}/${filename}"

    # Create temp directory
    local tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" EXIT

    # Download
    download "$url" "${tmpdir}/${filename}"

    # Extract
    info "Extracting..."
    tar -xzf "${tmpdir}/${filename}" -C "$tmpdir"

    # Install
    info "Installing to ${INSTALL_DIR}..."

    if [[ -w "$INSTALL_DIR" ]]; then
        mv "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
        chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    else
        warn "Need sudo to install to ${INSTALL_DIR}"
        sudo mv "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
        sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    # Verify
    if command -v "$BINARY_NAME" &> /dev/null; then
        success "Successfully installed ${BINARY_NAME} ${version}"
        echo ""
        info "Run '${BINARY_NAME} --help' to get started"
    else
        warn "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
        warn "Make sure ${INSTALL_DIR} is in your PATH"
    fi

    echo ""
}

main "$@"

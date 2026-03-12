#!/bin/bash
set -e

# Parse command line arguments
VERBOSE=false
for arg in "$@"; do
    case $arg in
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        *)
            # Unknown option
            ;;
    esac
done

# Verbose logging function
log_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo "[VERBOSE] $*" >&2
    fi
}

log_verbose "=== STARTING DIAGNOSTIC MODE ==="
log_verbose "Script arguments: $*"
log_verbose "VERBOSE mode enabled"

# Ensure target directory exists and cd into it
mkdir -p ~/.local/bin && cd ~/.local/bin

# Determine OS and Architecture
os=$(uname -s | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)

log_verbose "=== SYSTEM DETECTION ==="
log_verbose "Raw OS from uname -s: $(uname -s)"
log_verbose "Normalized OS: $os"
log_verbose "Raw architecture from uname -m: $(uname -m)"
log_verbose "Original arch: $arch"

if [ "$arch" = "arm64" ]; then 
    arch="aarch64"
    log_verbose "Converted arm64 to aarch64"
elif [ "$arch" = "x86_64" ]; then 
    arch="x86_64"
    log_verbose "Keeping x86_64 as is"
else
    log_verbose "Architecture $arch not explicitly handled, keeping as is"
fi

log_verbose "Final architecture: $arch"

# Determine the appropriate release asset based on OS
log_verbose "=== TARGET SELECTION ==="

if [ "$os" != "linux" ]; then
    echo "Unsupported OS: $os. The current installer only supports Linux releases." >&2
    exit 1
fi

if [ "$arch" = "x86_64" ]; then
    asset_suffix="linux_amd64"
    echo "Detected Linux x86_64, using linux_amd64 release asset"
    log_verbose "Linux x86_64 detected, selected asset suffix: $asset_suffix"
else
    echo "Unsupported architecture: $arch. The current installer only supports x86_64 Linux releases." >&2
    exit 1
fi

log_verbose "Final asset suffix: $asset_suffix"

# Fetch the latest release data from GitHub API and extract the download URL for the matching asset
echo "Fetching download URL for $asset_suffix..."
log_verbose "=== GITHUB API QUERY ==="
log_verbose "Fetching from: https://api.github.com/repos/a2-ai/pharos/releases/latest"

github_response=$(curl -s https://api.github.com/repos/a2-ai/pharos/releases/latest)
log_verbose "GitHub API response length: $(echo "$github_response" | wc -c) characters"

if [ "$VERBOSE" = true ]; then
    log_verbose "Available assets in release:"
    echo "$github_response" | grep -o '"browser_download_url": "[^"]*"' | sed 's/"browser_download_url": "//; s/"//' | while read -r url; do
        log_verbose "  - $(basename "$url")"
    done
fi

asset_url=$(echo "$github_response" | grep -o "https://github.com/A2-ai/pharos/releases/download/.*pharos_.*_${asset_suffix}\.tar\.gz")
log_verbose "Searching for pattern: *pharos_*_${asset_suffix}.tar.gz"
log_verbose "Found asset URL: $asset_url"

# Check if URL was found
if [ -z "$asset_url" ]; then
    echo "Error: Could not find a suitable release asset for your system ($asset_suffix) on GitHub." >&2
    echo "Please check available assets at https://github.com/a2-ai/pharos/releases/latest" >&2
    echo "Available targets typically include:" >&2
    echo "  - pharos_<version>_linux_amd64.tar.gz" >&2
    
    if [ "$VERBOSE" = true ]; then
        log_verbose "=== DEBUGGING INFO ==="
        log_verbose "Asset suffix we searched for: $asset_suffix"
        log_verbose "All download URLs found in response:"
        echo "$github_response" | grep -o '"browser_download_url": "[^"]*"' | sed 's/"browser_download_url": "//; s/"//' | while read -r url; do
            log_verbose "  $url"
        done
    fi
    
    exit 1
fi

log_verbose "=== DOWNLOAD AND INSTALLATION ==="
log_verbose "Download URL: $asset_url"

# Download the asset using curl, extract it, clean up, and make executable
echo "Downloading pharos from $asset_url"
curl -L -o pharos_latest.tar.gz "$asset_url" &&
    tar -xzf pharos_latest.tar.gz &&
    rm pharos_latest.tar.gz &&
    chmod +x pharos &&
    echo "pharos installed successfully to ~/.local/bin" ||
    (echo "Installation failed." >&2 && exit 1)

log_verbose "Installation completed successfully"

# Add ~/.local/bin to PATH if not already present
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "Adding ~/.local/bin to your PATH..."
    log_verbose "~/.local/bin not in PATH, adding it"
    if [[ "$SHELL" == *"bash"* ]]; then
        printf '\n%s\n' 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
        echo "Please source ~/.bashrc or open a new terminal."
        log_verbose "Added to ~/.bashrc"
    elif [[ "$SHELL" == *"zsh"* ]]; then
        printf '\n%s\n' 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
        echo "Please source ~/.zshrc or open a new terminal."
        log_verbose "Added to ~/.zshrc"
    elif [[ "$SHELL" == *"fish"* ]]; then
        mkdir -p ~/.config/fish
        printf '\n%s\n' 'fish_add_path "$HOME/.local/bin"' >> ~/.config/fish/config.fish
        echo "~/.local/bin added to fish path. Changes will apply to new fish shells."
        log_verbose "Added to fish config"
    else
        echo "Could not detect shell. Please add ~/.local/bin to your PATH manually."
        log_verbose "Unknown shell: $SHELL"
    fi
else
    echo "~/.local/bin is already in your PATH."
    log_verbose "~/.local/bin already in PATH"
fi

log_verbose "=== SCRIPT COMPLETED ==="

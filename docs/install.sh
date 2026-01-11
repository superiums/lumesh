#!/bin/bash
# Lumesh Installation Script
# Automatically installs lume, lume-se, documentation, and creates symlink

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CODEBERG_REPO="santo/lumesh"
GITHUB_REPO="superiums/lumesh"
INSTALL_DIR="$HOME/.local/bin"  # Default to user installation
CONFIG_DIR="$HOME/.config/lumesh"
DOC_DIR="$HOME/.local/share/lumesh"
SYSTEM_INSTALL_DIR="/usr/local/bin"
# Use sudo for system installation if needed
sudo_cmd=""

# Ask for installation type
ask_install_type() {
    echo -e "${YELLOW}Choose installation type:${NC}"
    echo "1) User installation (recommended) - installs to ~/.local/bin"
    echo "2) System installation - requires sudo, installs to /usr/local/bin"
    echo ""
    read -p "Enter choice (1-2) [1]: " choice
    choice=${choice:-1}

    case $choice in
        1)
            INSTALL_DIR="$HOME/.local/bin"
            CONFIG_DIR="$HOME/.config/lumesh"
            DOC_DIR="$HOME/.local/share/lumesh/doc"
            echo -e "${GREEN}User installation selected${NC}"
            ;;
        2)
            INSTALL_DIR="$SYSTEM_INSTALL_DIR"
            CONFIG_DIR="/etc/lumesh"
            DOC_DIR="/usr/local/share/lumesh/doc"
            echo -e "${GREEN}System installation selected${NC}"
            echo -e "${YELLOW}Note: This will require sudo privileges${NC}"
            if [ "$(id -u)" -ne 0 ]; then
                if command -v sudo >/dev/null 2>&1; then
                    sudo_cmd="sudo"
                elif command -v doas >/dev/null 2>&1; then
                    sudo_cmd="doas"
                fi
            fi
            ;;
        *)
            echo -e "${RED}Invalid choice. Defaulting to user installation.${NC}"
            INSTALL_DIR="$HOME/.local/bin"
            CONFIG_DIR="$HOME/.config/lumesh"
            DOC_DIR="$HOME/.local/share/lumesh/doc"
            ;;
    esac
}

# Platform detection
detect_platform() {
    case "$(uname -s)" in
        Linux*)
            PLATFORM="linux"
            # Detect libc variant
            if ldd --version 2>&1 | grep -q musl; then
                LIBC="musl"
            else
                LIBC="libc"
            fi
            ;;
        Darwin*)
            PLATFORM="macos"
            LIBC="libc"
            ;;
        CYGWIN*|MINGW*|MSYS*)
            PLATFORM="windows"
            LIBC="libc"
            ;;
        *)
            echo -e "${RED}Unsupported platform${NC}"
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64)     ARCH="x86_64" ;;
        aarch64|arm64) ARCH="arm64" ;;
        *)          echo -e "${RED}Unsupported architecture${NC}"; exit 1 ;;
    esac
}

# Get latest version from releases API
get_latest_version() {
    echo -e "${BLUE}Fetching latest version...${NC}"

    if [ "$PLATFORM" = "macos" ]; then
        LATEST_VERSION=$(curl -s "https://api.github.com/repos/$GITHUB_REPO/releases/latest" | \
            grep -o '"tag_name": *"[^"]*"' | cut -d'"' -f4 | sed 's/^v//')
    else
        LATEST_VERSION=$(curl -s "https://codeberg.org/api/v1/repos/$CODEBERG_REPO/releases/latest" | \
            grep -o '"tag_name": *"[^"]*"' | cut -d'"' -f4 | sed 's/^v//')
    fi

    if [ -z "$LATEST_VERSION" ]; then
        echo -e "${RED}Failed to fetch latest version${NC}"
        exit 1
    fi

    echo -e "${GREEN}Latest version: $LATEST_VERSION${NC}"
}

# Download binaries from Codeberg
download_from_codeberg() {
    local binary_name="$1"
    local platform_suffix="$2"

    local download_url="https://codeberg.org/$CODEBERG_REPO/releases/download/v$LATEST_VERSION/$binary_name-$platform_suffix"

    echo -e "${BLUE}Downloading $binary_name from Codeberg...${NC}"


    if command -v curl >/dev/null 2>&1; then
        $sudo_cmd curl -L -o "$INSTALL_DIR/$binary_name" "$download_url"
    elif command -v wget >/dev/null 2>&1; then
        $sudo_cmd wget -O "$INSTALL_DIR/$binary_name" "$download_url"
    else
        echo -e "${RED}Neither curl nor wget found${NC}"
        exit 1
    fi

    if [ "$PLATFORM" != "windows" ]; then
        $sudo_cmd chmod +x "$INSTALL_DIR/$binary_name"
    fi

    echo -e "${GREEN}Downloaded to: $INSTALL_DIR/$binary_name${NC}"
}

# Download binaries from GitHub (for macOS)
download_from_github() {
    local binary_name="$1"
    local platform_suffix="$2"

    local download_url="https://github.com/$GITHUB_REPO/releases/download/v$LATEST_VERSION/${binary_name}_${platform_suffix}_v${LATEST_VERSION}"

    echo -e "${BLUE}Downloading $binary_name from GitHub...${NC}"

    # Use sudo for system installation if needed
    if command -v curl >/dev/null 2>&1; then
        $sudo_cmd curl -L -o "$INSTALL_DIR/$binary_name" "$download_url"
    elif command -v wget >/dev/null 2>&1; then
        $sudo_cmd wget -O "$INSTALL_DIR/$binary_name" "$download_url"
    else
        echo -e "${RED}Neither curl nor wget found${NC}"
        exit 1
    fi

    $sudo_cmd chmod +x "$INSTALL_DIR/$binary_name"
    echo -e "${GREEN}Downloaded to: $INSTALL_DIR/$binary_name${NC}"
}

# Download both binaries
download_binaries() {
    # Create install directory with appropriate permissions
    if [ "$INSTALL_DIR" = "$SYSTEM_INSTALL_DIR" ]; then
        if [ "$(id -u)" -ne 0 ]; then
            echo -e "${BLUE}Creating system directory with sudo...${NC}"
            $sudo_cmd mkdir -p "$INSTALL_DIR"
        else
            mkdir -p "$INSTALL_DIR"
        fi
    else
        mkdir -p "$INSTALL_DIR"
    fi

    if [ "$PLATFORM" = "macos" ]; then
        download_from_github "lume" "macos"
        download_from_github "lume-se" "macos"
    elif [ "$PLATFORM" = "windows" ]; then
        download_from_codeberg "lume" "windows.exe"
        download_from_codeberg "lume-se" "windows.exe"
    elif [ "$PLATFORM" = "linux" ]; then
        if [ "$LIBC" = "musl" ]; then
            download_from_codeberg "lume" "linux-musl"
            download_from_codeberg "lume-se" "linux-musl"
        else
            download_from_codeberg "lume" "linux"
            download_from_codeberg "lume-se" "linux"
        fi
    fi
}

# Create symlink from lume-se to lumesh
create_symlink() {
    echo -e "${BLUE}Creating symlink from lume-se to lumesh...${NC}"

    local lume_path="$INSTALL_DIR/lume"
    local lumesh_link="$INSTALL_DIR/lumesh"

    # Use sudo for system installation if needed
    # Remove existing link if it exists
    if [ -L "$lumesh_link" ]; then
        $sudo_cmd rm "$lumesh_link"
    elif [ -f "$lumesh_link" ]; then
        echo -e "${YELLOW}Warning: $lumesh_link exists and is not a symlink. Skipping symlink creation.${NC}"
        return
    fi

    # Create platform-specific symlink
    if [ "$PLATFORM" = "windows" ]; then
        # On Windows, use mklink via cmd
        if command -v cmd.exe >/dev/null 2>&1; then
            cmd.exe /c "mklink \"$lumesh_link\" \"$lume_path\"" >/dev/null 2>&1
            if [ $? -eq 0 ]; then
                echo -e "${GREEN}Created symlink: $lumesh_link -> $lume_path${NC}"
            else
                echo -e "${YELLOW}Failed to create symlink on Windows. You can manually create it if needed.${NC}"
            fi
        else
            echo -e "${YELLOW}Cannot create symlink on Windows without cmd.exe${NC}"
        fi
    else
        # On Unix-like systems, use ln -s
        $sudo_cmd ln -s "$lume_path" "$lumesh_link"
        echo -e "${GREEN}Created symlink: $lumesh_link -> $lume_path${NC}"
    fi
}

# Download and extract documentation
download_docs() {
    echo -e "${BLUE}Downloading documentation...${NC}"

    # Use sudo for system installation if needed
    $sudo_cmd mkdir -p "$DOC_DIR"
    local doc_url="https://codeberg.org/$CODEBERG_REPO/releases/download/v$LATEST_VERSION/doc.tar.gz"

    # Download to temp file first, then move with sudo if needed
    local temp_doc="/tmp/doc.tar.gz"
    if command -v curl >/dev/null 2>&1; then
        curl -L -o "$temp_doc" "$doc_url"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$temp_doc" "$doc_url"
    fi

    # Extract and move to final location
    cd /tmp
    tar -xzf "$temp_doc"
    $sudo_cmd cp -r doc/install/* "$DOC_DIR/"
    rm -rf doc "$temp_doc"

    echo -e "${GREEN}Documentation extracted to: $DOC_DIR${NC}"
}

# Setup PATH
setup_path() {
    if [ "$PLATFORM" = "windows" ]; then
        echo -e "${YELLOW}Please add $INSTALL_DIR to your PATH manually${NC}"
        return
    fi

    # For system installation, /usr/local/bin should already be in PATH
    if [ "$INSTALL_DIR" = "$SYSTEM_INSTALL_DIR" ]; then
        if echo "$PATH" | grep -q "$INSTALL_DIR"; then
            echo -e "${GREEN}$INSTALL_DIR is already in PATH${NC}"
        else
            echo -e "${YELLOW}Warning: $INSTALL_DIR is not in PATH. You may need to add it manually.${NC}"
        fi
        return
    fi

    # User installation PATH setup
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo -e "${GREEN}$INSTALL_DIR is already in PATH${NC}"
        return
    fi

    local shell_profile=""
    case "$SHELL" in
        */bash) shell_profile="$HOME/.bashrc" ;;
        */zsh)  shell_profile="$HOME/.zshrc" ;;
        */fish) shell_profile="$HOME/.config/fish/config.fish" ;;
        *)      shell_profile="$HOME/.profile" ;;
    esac

    echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$shell_profile"
    echo -e "${GREEN}Added $INSTALL_DIR to PATH in $shell_profile${NC}"
    echo -e "${YELLOW}Please restart your shell or run: source $shell_profile${NC}"
}

# Add Lumesh to system shells list for chsh usage
add_to_shell_list() {
    local lume_path="$1"

    # Check if lume path exists
    if [ ! -f "$lume_path" ]; then
        echo -e "${RED}Error: Lumesh binary not found at $lume_path${NC}"
        return 1
    fi

    # Check if already in /etc/shells
    if [ -f /etc/shells ] && grep -q "^$lume_path$" /etc/shells; then
        echo -e "${GREEN}Lumesh is already in /etc/shells${NC}"
    else
        echo -e "${BLUE}Adding Lumesh to /etc/shells...${NC}"
        # Use sudo or doas to append to /etc/shells
        if command -v sudo >/dev/null 2>&1; then
            echo "$lume_path" | sudo tee -a /etc/shells >/dev/null
        elif command -v doas >/dev/null 2>&1; then
            echo "$lume_path" | doas tee -a /etc/shells >/dev/null
        else
            echo -e "${RED}Error: Need sudo or doas to modify /etc/shells${NC}"
            echo -e "${YELLOW}Please manually add '$lume_path' to /etc/shells${NC}"
            return 1
        fi
        echo -e "${GREEN}Added $lume_path to /etc/shells${NC}"
    fi

    # Ask if user wants to change shell now
    echo ""
    echo -e "${YELLOW}Would you like to set Lumesh as your default login shell now?${NC}"
    echo "This will change your login shell to: $lume_path"
    read -p "Change shell? (y/N) " change_shell

    if [[ "$change_shell" =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Changing login shell...${NC}"
        if command -v sudo >/dev/null 2>&1; then
            sudo chsh -s "$lume_path"
        elif command -v doas >/dev/null 2>&1; then
            doas chsh -s "$lume_path"
        else
            chsh -s "$lume_path"
        fi
        echo -e "${GREEN}Login shell changed to Lumesh${NC}"
        echo -e "${YELLOW}Note: Changes will take effect on next login${NC}"
    else
        echo -e "${BLUE}You can change your shell later with: chsh -s $lume_path${NC}"
    fi
}

# Main installation
main() {
    echo -e "${BLUE}Lumesh Installation Script${NC}"
    echo "=================================="

    # Ask for installation type first
    ask_install_type
    echo ""

    detect_platform
    echo -e "${GREEN}Detected platform: $PLATFORM-$ARCH ($LIBC)${NC}"

    get_latest_version

    download_binaries
    create_symlink
    download_docs
    setup_path

    # Offer to add to shell list
    if [ "$PLATFORM" != "windows" ]; then
        echo ""
        read -p "Would you like to add Lumesh to system shell list for chsh? (y/N) " add_shell
        if [[ "$add_shell" =~ ^[Yy]$ ]]; then
            add_to_shell_list "$INSTALL_DIR/lume"
        fi
    fi

    echo ""
    echo -e "${GREEN}Installation completed successfully!${NC}"
    echo -e "${BLUE}Installation location: $INSTALL_DIR${NC}"
    echo -e "${BLUE}To start using Lumesh:${NC}"
    echo "  # Start interactive shell"
    echo "  lume"
    echo ""
    echo "  # Or execute a script"
    echo "  lumesh script.lm"
    echo ""
    echo -e "${BLUE}For more information, see:${NC}"
    echo "  https://lumesh.codeberg.page/"
    echo "Type 'doc' in lume to open doc."

}

main "$@"

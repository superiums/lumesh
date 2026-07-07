#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="${CONFIG_DIR:-$HOME/.config/lumesh}"
DOC_DIR="${DOC_DIR:-$HOME/.local/share}"
SYSTEM_INSTALL_DIR="/usr/local/bin"
sudo_cmd=""

# Detect platform from binary name
LUME_BIN=$(ls "$SCRIPT_DIR"/lume-* 2>/dev/null | grep -v 'lume-se' | head -1)
if [ -z "$LUME_BIN" ]; then
    echo "Error: No lume binary found in $SCRIPT_DIR"
    exit 1
fi

TARGET=$(basename "$LUME_BIN" | sed 's/^lume-//')
case "$TARGET" in
    *-linux-*)   PLATFORM="linux" ;;
    *-darwin)    PLATFORM="darwin" ;;
    *-windows-*) PLATFORM="windows" ;;
    *-freebsd)   PLATFORM="freebsd" ;;
    *)           echo "Unknown platform: $TARGET"; exit 1 ;;
esac

# macOS paths
if [ "$PLATFORM" = "darwin" ]; then
    CONFIG_DIR="$HOME/Library/Application Support/lumesh"
    DOC_DIR="$HOME/Library/Application Support"
fi

ask_install_type() {
    echo "Choose installation type:"
    echo "1) User installation (recommended) - installs to $([ "$PLATFORM" = "darwin" ] && echo '$HOME/.local/bin' || echo '~/.local/bin')"
    echo "2) System installation - requires sudo, installs to /usr/local/bin"
    read -p "Enter choice (1-2) [1]: " choice
    choice=${choice:-1}
    if [ "$choice" = "2" ]; then
        INSTALL_DIR="$SYSTEM_INSTALL_DIR"
        if [ "$PLATFORM" = "darwin" ]; then
            DOC_DIR="/Library/Application Support"
        else
            DOC_DIR="/usr/local/share"
        fi
        if [ "$(id -u)" -ne 0 ]; then
            if command -v sudo >/dev/null 2>&1; then
                sudo_cmd="sudo"
            elif command -v doas >/dev/null 2>&1; then
                sudo_cmd="doas"
            fi
        fi
    fi
}

install_binaries() {
    local lume_se_bin=$(ls "$SCRIPT_DIR"/lume-se-* 2>/dev/null | head -1)
    $sudo_cmd mkdir -p "$INSTALL_DIR"
    install -m 755 "$LUME_BIN" "$INSTALL_DIR/lume"
    if [ -n "$lume_se_bin" ]; then
        install -m 755 "$lume_se_bin" "$INSTALL_DIR/lume-se"
    fi
    if [ "$PLATFORM" != "windows" ]; then
        $sudo_cmd ln -sf "$INSTALL_DIR/lume" "$INSTALL_DIR/lumesh"
    fi
    echo "Installed lume to: $INSTALL_DIR/lume"
    [ -n "$lume_se_bin" ] && echo "Installed lume-se to: $INSTALL_DIR/lume-se"
}

install_data() {
    if [ ! -d "$SCRIPT_DIR/lumesh" ]; then
        echo "No data directory found, skipping."
        return
    fi
    $sudo_cmd mkdir -p "$DOC_DIR"
    $sudo_cmd cp -r "$SCRIPT_DIR/lumesh" "$DOC_DIR/"
    if [ -d "$DOC_DIR/lumesh/examples" ]; then
        $sudo_cmd mkdir -p "$CONFIG_DIR"
        $sudo_cmd cp -f "$DOC_DIR/lumesh/examples/config.lm" "$CONFIG_DIR/" 2>/dev/null || true
        for f in "$DOC_DIR"/lumesh/examples/* ; do
            [ -f "$f" ] && $sudo_cmd cp -f "$f" "$CONFIG_DIR/" 2>/dev/null || true
        done
    fi
    echo "Documentation installed to: $DOC_DIR"
}

setup_path() {
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
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
    echo "Added $INSTALL_DIR to PATH in $shell_profile"
    echo "Restart your shell or run: source $shell_profile"
}

echo "Lume ($TARGET) Installer"
echo "========================"
ask_install_type
echo ""
install_binaries
install_data
setup_path
echo ""
echo "Installation complete!"
echo "  lume: $INSTALL_DIR/lume"
echo ""
echo "To start using Lumesh:"
echo "  lume"
echo ""
echo "Documentation: $DOC_DIR/lumesh"

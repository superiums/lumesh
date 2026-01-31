#!/bin/bash  
# Lumesh GitHub Installation Script  
# Downloads binaries from GitHub releases and installs to user or system  
  
set -e  
  
# Colors for output  
RED='\033[0;31m'  
GREEN='\033[0;32m'  
YELLOW='\033[1;33m'  
BLUE='\033[0;34m'  
NC='\033[0m' # No Color  
  
# Configuration  
GITHUB_REPO="superiums/lumesh"  
INSTALL_DIR="$HOME/.local/bin"  # Default to user installation  
CONFIG_DIR="$HOME/.config/lumesh"  
DOC_DIR="$HOME/.local/share"  
SYSTEM_INSTALL_DIR="/usr/local/bin"  
sudo_cmd=""  
  
# Platform detection  
detect_platform() {  
    case "$(uname -s)" in  
        Linux*)  
            PLATFORM="linux"  
            # Detect libc variant  
            if ldd --version 2>&1 | grep -q musl; then  
                LIBC="musl"  
            else  
                LIBC="gnu"  
            fi  
            ;;  
        Darwin*)  
            PLATFORM="darwin"  
            LIBC="libc"  
            ;;  
        CYGWIN*|MINGW*|MSYS*)  
            PLATFORM="windows"  
            LIBC="libc"  
            ;;  
        FreeBSD*)  
            PLATFORM="freebsd"  
            LIBC="libc"  
            ;;  
        *)  
            echo -e "${RED}Unsupported platform: $(uname -s)${NC}"  
            exit 1  
            ;;  
    esac  
  
    case "$(uname -m)" in  
        x86_64)     ARCH="x86_64" ;;  
        aarch64|arm64) ARCH="aarch64" ;;  
        *)          echo -e "${RED}Unsupported architecture: $(uname -m)${NC}"; exit 1 ;;  
    esac  
}  
  
# Get platform-specific asset name  
get_asset_name() {  
    case "$PLATFORM" in  
        linux)  
            if [ "$LIBC" = "musl" ]; then  
                echo "lume-x86_64-linux-musl"  
            else  
                echo "lume-x86_64-linux-gnu"  
            fi  
            ;;  
        darwin)  
            if [ "$ARCH" = "aarch64" ]; then  
                echo "lume-aarch64-apple-darwin"  
            else  
                echo "lume-x86_64-apple-darwin"  
            fi  
            ;;  
        windows)  
            echo "lume-x86_64-pc-windows-gnu.exe"  
            ;;  
        freebsd)  
            echo "lume-x86_64-freebsd"  
            ;;  
        android)  
            echo "lume-aarch64-linux-android"  
            ;;  
    esac  
}  
  
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
            echo -e "${GREEN}User installation selected${NC}"  
            ;;  
        2)  
            INSTALL_DIR="$SYSTEM_INSTALL_DIR"  
            # CONFIG_DIR="/etc/lumesh"
            DOC_DIR="/usr/local/share"  
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
            ;;  
    esac  
}  
  
# Get latest version from GitHub API  
get_latest_version() {  
    echo -e "${BLUE}Fetching latest version...${NC}"  
      
    LATEST_VERSION=$(curl -s "https://api.github.com/repos/$GITHUB_REPO/releases/latest" | grep -o '"tag_name": *"[^"]*"' | cut -d'"' -f4 | sed 's/^c//')  
  
    if [ -z "$LATEST_VERSION" ]; then  
        echo -e "${RED}Failed to fetch latest version${NC}"  
        exit 1  
    fi  
  
    echo -e "${GREEN}Latest version: $LATEST_VERSION${NC}"  
}  
  
# Download binary from GitHub  
download_binary() {  
    local asset_name=$(get_asset_name)  
    local download_url="https://github.com/$GITHUB_REPO/releases/download/c$LATEST_VERSION/$asset_name"  
  
    echo -e "${BLUE}Downloading $asset_name...${NC}"  
  
    # Create install directory  
    if [ "$INSTALL_DIR" = "$SYSTEM_INSTALL_DIR" ]; then  
        if [ "$(id -u)" -ne 0 ]; then  
            $sudo_cmd mkdir -p "$INSTALL_DIR"  
        else  
            mkdir -p "$INSTALL_DIR"  
        fi  
    else  
        mkdir -p "$INSTALL_DIR"  
    fi  
  
      # 创建临时目录  
    local TEMP_DIR=$(mktemp -d)  
  
  # 根据平台处理  
    if [ "$PLATFORM" = "windows" ]; then  
        download_with_retry "$download_url" "$TEMP_DIR/lume.exe"  
        $sudo_cmd mv "$TEMP_DIR/lume.exe" "$INSTALL_DIR/"  
    else  
        download_with_retry "$download_url" "$TEMP_DIR/lume"  
        $sudo_cmd mv "$TEMP_DIR/lume" "$INSTALL_DIR/"  
    fi

    # 设置权限  
    if [ "$PLATFORM" != "windows" ]; then  
        $sudo_cmd chmod +x "$INSTALL_DIR/lume"  
    fi  
  
    # 清理临时目录  
    rm -rf "$TEMP_DIR"

    echo -e "${GREEN}Downloaded to: $INSTALL_DIR/lume${NC}"  
}  

  # 带重试和验证的下载函数  
download_with_retry() {  
    local url="$1"  
    local output="$2"  
    local max_retries=3  
    local retry_count=0  
    local temp_output="${output}.tmp"  
      
    # 清理可能存在的临时文件  
    rm -f "$temp_output"  
      
    while [ $retry_count -lt $max_retries ]; do  
        echo -e "${BLUE}Downloading (attempt $((retry_count + 1))/$max_retries)...${NC}"  
          
        if command -v curl >/dev/null 2>&1; then  
            # 使用断点续传和进度显示  
            if curl -L -C - --progress-bar -o "$temp_output" "$url"; then  
                break  
            fi  
        elif command -v wget >/dev/null 2>&1; then  
            # 使用断点续传  
            if wget -c --progress=bar:force -O "$temp_output" "$url" 2>&1; then  
                break  
            fi  
        else  
            echo -e "${RED}Neither curl nor wget found${NC}"  
            return 1  
        fi  
          
        retry_count=$((retry_count + 1))  
        if [ $retry_count -lt $max_retries ]; then  
            echo -e "${YELLOW}Download failed, retrying in 5 seconds...${NC}"  
            sleep 5  
        fi  
    done  
      
    if [ $retry_count -eq $max_retries ]; then  
        echo -e "${RED}Download failed after $max_retries attempts${NC}"  
        rm -f "$temp_output"  
        return 1  
    fi  
      
    # 验证文件大小（基本检查）  
    if [ ! -s "$temp_output" ]; then  
        echo -e "${RED}Downloaded file is empty${NC}"  
        rm -f "$temp_output"  
        return 1  
    fi  
      
    # 移动到最终位置  
    mv "$temp_output" "$output"  
    echo -e "${GREEN}Download completed successfully${NC}"  
    return 0  
}


# Download and extract data.tgz for non-Windows platforms  
download_data() {  
    if [ "$PLATFORM" = "windows" ]; then  
        echo -e "${YELLOW}Skipping data.tgz download on Windows${NC}"  
        return  
    fi  
  
    echo -e "${BLUE}Downloading data.tgz...${NC}"  
      
    local data_url="https://github.com/$GITHUB_REPO/releases/download/c$LATEST_VERSION/data.tgz"  
    local temp_data="/tmp/data.tgz"  
  
    # Download data.tgz  
    # if command -v curl >/dev/null 2>&1; then  
    #     curl -L -o "$temp_data" "$data_url"  
    # elif command -v wget >/dev/null 2>&1; then  
    #     wget -O "$temp_data" "$data_url"  
    # fi  
  
    download_with_retry "$data_url" "$temp_data"  
        
    # Create share directory and extract  
    $sudo_cmd mkdir -p "$DOC_DIR"  
    $sudo_cmd mkdir -p "$CONFIG_DIR"

    cd /tmp  
    $sudo_cmd tar -xzf "$temp_data" -C "$DOC_DIR"  
    if [ -d "$DOC_DIR/lumesh/examples" ]; then  
        cp "$DOC_DIR/lumesh/examples/config.lm" "$CONFIG_DIR/"  
        cp "$DOC_DIR/lumesh/examples/prompt*.lm" "$CONFIG_DIR/" 2>/dev/null || true  
    fi
    rm "$temp_data"  
  
    echo -e "${GREEN}Data extracted to: $DOC_DIR${NC}"  
}  
  
# Create symlink from lume to lumesh  
create_symlink() {  
    echo -e "${BLUE}Creating symlink from lume to lumesh...${NC}"  
  
    local lume_path="$INSTALL_DIR/lume"  
    local lumesh_link="$INSTALL_DIR/lumesh"  
  
    # Remove existing link if it exists  
    if [ -L "$lumesh_link" ]; then  
        $sudo_cmd rm "$lumesh_link"  
    elif [ -f "$lumesh_link" ]; then  
        echo -e "${YELLOW}Warning: $lumesh_link exists and is not a symlink. Skipping symlink creation.${NC}"  
        return  
    fi  
  
    # Create symlink  
    $sudo_cmd ln -s "$lume_path" "$lumesh_link"  
    echo -e "${GREEN}Created symlink: $lumesh_link -> $lume_path${NC}"  
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
        echo "$lume_path" | $sudo_cmd tee -a /etc/shells >/dev/null  
        echo -e "${GREEN}Added $lume_path to /etc/shells${NC}"  
    fi  
  
    # Ask if user wants to change shell now  
    echo ""  
    echo -e "${YELLOW}Would you like to set Lumesh as your default login shell now?${NC}"  
    echo "This will change your login shell to: $lume_path"  
    read -p "Change shell? (y/N) " change_shell  
  
    if [[ "$change_shell" =~ ^[Yy]$ ]]; then  
        echo -e "${BLUE}Changing login shell...${NC}"  
        $sudo_cmd chsh -s "$lume_path"  
        echo -e "${GREEN}Login shell changed to Lumesh${NC}"  
        echo -e "${YELLOW}Note: Changes will take effect on next login${NC}"  
    else  
        echo -e "${BLUE}You can change your shell later with: chsh -s $lume_path${NC}"  
    fi  
}  

# Configure Helix editor for tree-sitter-lumesh syntax highlighting  
configure_helix_lumesh() {  
    echo -e "${BLUE}Checking Grammar Highlight Config...${NC}"  
    
    local HELIX_CONFIG="$HOME/.config/helix"  
    local HELIX_RUNTIME="$HELIX_CONFIG/runtime"  
      
    # Detect if Helix is installed  
    if ! command -v hx >/dev/null 2>&1 && [[ ! -d "$HELIX_CONFIG" ]]; then  
        echo "❌ Helix editor not detected"  
        return 1  
    fi  
      
    echo "✅ Helix editor detected, starting configuration..."  
      
    # Check if source files exist  
    local GRAMMAR_SO="$DOC_DIR/lumesh/tree-sitter-lumesh/grammars/lumesh.so"  
    local QUERIES_DIR="$DOC_DIR/lumesh/tree-sitter-lumesh/queries/lumesh"  
      
    if [[ ! -f "$GRAMMAR_SO" ]]; then  
        echo "❌ Grammar file not found: $GRAMMAR_SO"  
        return 1  
    fi  
      
    if [[ ! -d "$QUERIES_DIR" ]]; then  
        echo "❌ Queries directory not found: $QUERIES_DIR"  
        return 1  
    fi  
      
    # Create runtime directories  
    mkdir -p "$HELIX_RUNTIME/grammars"  
    mkdir -p "$HELIX_RUNTIME/queries"  
      
    # Create symbolic links  
    echo "🔗 Creating grammar file symlink..."  
    ln -sf "$GRAMMAR_SO" "$HELIX_RUNTIME/grammars/lumesh.so"  
      
    echo "🔗 Creating queries directory symlink..."  
    ln -sf "$QUERIES_DIR" "$HELIX_RUNTIME/queries/lumesh"  
      
    # Create languages.toml configuration  
    local LANG_FILE="$HELIX_CONFIG/languages.toml"  
      
    # Check if configuration already exists  
    if ! grep -q "name = \"lumesh\"" "$LANG_FILE" 2>/dev/null; then  
        echo "📝 Adding language configuration..."  
        cat >> "$LANG_FILE" << 'EOF'  
  
[[language]]  
name = "lumesh"  
scope = "source.lumesh"  
injection-regex = "lumesh"  
file-types = ["lm", "lumesh"]  
shebangs = ["lume","lumesh"]  
roots = []  
comment-token = "#"  
indent = { tab-width = 2, unit = "  " }  
EOF  
    else  
        echo "ℹ️  Language configuration already exists"  
    fi  
      
    echo "✅ Helix configuration completed!"  
    echo ""  
    echo "📋 Manual configuration instructions:"  
    echo "1. Restart Helix editor to load new syntax"  
    echo "2. If syntax highlighting is not working, check $LANG_FILE configuration"  
    echo "3. Ensure $HELIX_RUNTIME/grammars/lumesh.so symlink is valid"  
    echo ""  
    echo "🔍 Verification commands:"  
    echo "   ls -la $HELIX_RUNTIME/grammars/lumesh.so"  
    echo "   ls -la $HELIX_RUNTIME/queries/lumesh"  
}

# Main installation  
main() {  
    echo -e "${BLUE}Lumesh GitHub Installation Script${NC}"  
    echo "======================================"  
  
    # Ask for installation type first  
    ask_install_type  
    echo ""  
  
    detect_platform  
    echo -e "${GREEN}Detected platform: $PLATFORM-$ARCH ($LIBC)${NC}"  
  
    get_latest_version  
    download_binary  
    download_data  
    create_symlink  
    setup_path  
  
    configure_helix_lumesh

    # Offer to add to shell list for system installation  
    if [ "$INSTALL_DIR" = "$SYSTEM_INSTALL_DIR" ] && [ "$PLATFORM" != "windows" ]; then  
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
    echo "  https://github.com/$GITHUB_REPO/"  
}  
  
main "$@"

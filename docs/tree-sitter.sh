#!/usr/bin/env bash
# tree-sitter-lumesh WASM installation script with symlink optimization
# Supports: Neovim, VS Code, Helix, Zed, Emacs, Sublime Text

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_OWNER="superiums"
REPO_NAME="tree-sitter-lumesh"
API_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
INSTALL_DIR="$HOME/.local/share/tree-sitter-lumesh"
WASM_FILE="$INSTALL_DIR/lumesh.wasm"
QUERIES_DIR="$INSTALL_DIR/queries"

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Detect OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "linux"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macos"
    elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
        echo "windows"
    else
        echo "unknown"
    fi
}

# Create symlink (cross-platform)
create_symlink() {
    local source="$1"
    local target="$2"

    # Remove existing target if it exists
    if [[ -e "$target" ]] || [[ -L "$target" ]]; then
        rm -rf "$target"
    fi

    # Create parent directory if needed
    mkdir -p "$(dirname "$target")"

    # Create symlink based on OS
    OS=$(detect_os)
    case $OS in
        "windows")
            # On Windows, use mklink for proper symlinks
            if [[ -d "$source" ]]; then
                # Directory symlink
                cmd /c "mklink /J \"$(cygpath -w "$target")\" \"$(cygpath -w "$source")\"" 2>/dev/null || {
                    log_warning "Failed to create directory symlink, copying instead"
                    cp -r "$source" "$target"
                }
            else
                # File symlink
                cmd /c "mklink \"$(cygpath -w "$target")\" \"$(cygpath -w "$source")\"" 2>/dev/null || {
                    log_warning "Failed to create file symlink, copying instead"
                    cp "$source" "$target"
                }
            fi
            ;;
        *)
            # Unix-like systems use ln -s
            ln -sf "$source" "$target"
            ;;
    esac
}

# Download WASM and tar.gz files from GitHub releases
download_files() {
    log_info "Downloading tree-sitter-lumesh files..."

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Get latest release info
    if command_exists curl; then
        RELEASE_INFO=$(curl -s "$API_URL")
    elif command_exists wget; then
        RELEASE_INFO=$(wget -qO- "$API_URL")
    else
        log_error "curl or wget is required to download files"
        exit 1
    fi

    # Extract download URLs
    WASM_URL=$(echo "$RELEASE_INFO" | grep -o '"browser_download_url": "[^"]*\.wasm"' | cut -d'"' -f4 | head -1)
    TAR_URL=$(echo "$RELEASE_INFO" | grep -o '"browser_download_url": "[^"]*\.tar\.gz"' | cut -d'"' -f4 | head -1)

    if [[ -z "$WASM_URL" ]] || [[ -z "$TAR_URL" ]]; then
        log_error "Could not find required files in latest release"
        exit 1
    fi

    # Download WASM file
    log_info "Downloading WASM grammar file..."
    if command_exists curl; then
        curl -L -o "$WASM_FILE" "$WASM_URL"
    else
        wget -O "$WASM_FILE" "$WASM_URL"
    fi

    # Download and extract tar.gz
    log_info "Downloading and extracting queries..."
    OS=$(detect_os)
    case $OS in
        "windows")
            TEMP_TAR="$INSTALL_DIR/temp.tar.gz"
        ;;
        *)
            TEMP_TAR="/tmp/temp.tar.gz"
            cd /tmp
        ;;
    esac

    if command_exists curl; then
        curl -L -o "$TEMP_TAR" "$TAR_URL"
    else
        wget -O "$TEMP_TAR" "$TAR_URL"
    fi

    # Extract queries
    cd "$INSTALL_DIR"
    if command_exists tar; then
        tar -xzf "$TEMP_TAR"
        # Move queries to expected location if needed
        if [[ -d "tree-sitter-lumesh/queries" ]]; then
            mv "tree-sitter-lumesh/queries" "$QUERIES_DIR"
            rm -rf "tree-sitter-lumesh"
        elif [[ ! -d "$QUERIES_DIR" ]]; then
            mkdir -p "$QUERIES_DIR"
        fi
    else
        log_error "tar command not found, cannot extract queries"
        exit 1
    fi

    # Clean up
    rm -f "$TEMP_TAR"

    log_success "Files installed to $INSTALL_DIR"
}

# Configure Neovim with symlinks
configure_neovim() {
    if command_exists nvim; then
        log_info "Configuring Neovim with symlinks..."

        # Create symlinks
        create_symlink "$WASM_FILE" "$HOME/.config/nvim/lua/parser/lumesh.wasm"
        create_symlink "$QUERIES_DIR" "$HOME/.config/nvim/after/queries/lumesh"

        # Create nvim config
        cat > "$HOME/.config/nvim/after/plugin/lumesh.lua" << 'EOF'
-- Lumesh syntax highlighting with WASM
vim.filetype.add({
  extension = {
    lm = 'lumesh',
    lumesh = 'lumesh'
  },
  filename = {
    ['.lumeshrc'] = 'lumesh',
    ['config.lm'] = 'lumesh'
  }
})

vim.treesitter.language.register('lumesh', 'lumesh')
EOF

        log_success "Neovim configured with symlinks"
    else
        log_warning "Neovim not found, skipping..."
    fi
}

# Configure VS Code with symlinks
configure_vscode() {
    if command_exists code || [[ -d "$HOME/.vscode" ]] || [[ -d "$HOME/.vscode-server" ]]; then
        log_info "Configuring VS Code with symlinks..."

        # Create extension directory
        EXT_DIR="$HOME/.vscode/extensions/tree-sitter-lumesh-wasm"
        mkdir -p "$EXT_DIR"

        # Create symlinks
        create_symlink "$WASM_FILE" "$EXT_DIR/lumesh.wasm"
        create_symlink "$QUERIES_DIR" "$EXT_DIR/queries"

        # Create package.json
        cat > "$EXT_DIR/package.json" << 'EOF'
{
  "name": "tree-sitter-lumesh-wasm",
  "displayName": "Lumesh Syntax Highlighting (WASM)",
  "description": "Syntax highlighting for Lumesh shell language using WASM",
  "version": "0.1.0",
  "engines": {
    "vscode": "^1.0.0"
  },
  "categories": ["Programming Languages"],
  "contributes": {
    "languages": [{
      "id": "lumesh",
      "aliases": ["Lumesh", "lumesh"],
      "extensions": [".lm", ".lumesh"],
      "filenames": [".lumeshrc", "config.lm"],
      "configuration": "./language-configuration.json"
    }],
    "grammars": [{
      "language": "lumesh",
      "scopeName": "source.lumesh",
      "path": "./lumesh.wasm",
      "injectTo": ["source.shell"]
    }]
  }
}
EOF

        # Create language configuration
        cat > "$EXT_DIR/language-configuration.json" << 'EOF'
{
  "comments": {
    "lineComment": "#"
  },
  "brackets": [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"]
  ],
  "autoClosingPairs": [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
    ["\"", "\""],
    ["'", "'"]
  ]
}
EOF

        log_success "VS Code configured with symlinks"
        log_info "To install in VS Code:"
        log_info "1. Open VS Code"
        log_info "2. Go to Extensions -> Install from VSIX"
        log_info "3. Select the directory: $EXT_DIR"
    else
        log_warning "VS Code not found, skipping..."
    fi
}

# Configure Helix with symlinks
configure_helix() {
    if command_exists hx; then
        log_info "Configuring Helix with symlinks..."

        # Create symlinks
        create_symlink "$WASM_FILE" "$HOME/.local/share/helix/runtime/grammars/lumesh.wasm"
        create_symlink "$QUERIES_DIR" "$HOME/.local/share/helix/runtime/queries/lumesh"

        # Create languages.toml entry
        LANG_FILE="$HOME/.config/helix/languages.toml"
        mkdir -p "$(dirname "$LANG_FILE")"


            # Append to existing languages.toml
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


        log_success "Helix configured with symlinks"
    else
        log_warning "Helix not found, skipping..."
    fi
}

# Configure Zed with symlinks
# configure_zed() {
#     if command_exists zed || [[ -d "$HOME/.config/zed" ]]; then
#         log_info "Configuring Zed with symlinks..."

#         # Create symlinks
#         create_symlink "$WASM_FILE" "$HOME/.config/zed/grammars/lumesh.wasm"
#         create_symlink "$QUERIES_DIR" "$HOME/.config/zed/grammars/lumesh-queries"

#         # Create or update settings.json
#         SETTINGS_FILE="$HOME/.config/zed/settings.json"
#         if [[ ! -f "$SETTINGS_FILE" ]]; then
#             cat > "$SETTINGS_FILE" << 'EOF'
# {
#   "languages": [
#     {
#       "name": "Lumesh",
#       "language_id": "lumesh",
#       "extensions": [".lm", ".lumesh"],
#       "file_names": [".lumeshrc", "config.lm"],
#       "grammars": ["lumesh.wasm"]
#     }
#   ]
# }
# EOF
#         else
#             log_warning "Please manually add lumesh configuration to $SETTINGS_FILE"
#             log_info "Add the following to your languages array:"
#             cat << 'EOF'
# {
#   "name": "Lumesh",
#   "language_id": "lumesh",
#   "extensions": [".lm", ".lumesh"],
#   "file_names": [".lumeshrc", "config.lm"],
#   "grammars": ["lumesh.wasm"]
# }
# EOF
#         fi

#         log_success "Zed configured with symlinks"
#     else
#         log_warning "Zed editor not found, skipping..."
#         log_info "Zed editor supports WASM grammars. Install from https://zed.dev"
#     fi
# }

# Configure Emacs with symlinks
configure_emacs() {
    if command_exists emacs; then
        log_info "Configuring Emacs with symlinks..."

        # Create symlinks
        create_symlink "$WASM_FILE" "$HOME/.emacs.d/tree-sitter-langs/lumesh.wasm"
        create_symlink "$QUERIES_DIR" "$HOME/.emacs.d/tree-sitter-langs/lumesh-queries"

        # Add to emacs config
        cat >> "$HOME/.emacs.d/init.el" << 'EOF'

;; Tree-sitter Lumesh WASM configuration
(use-package tree-sitter
  :config
  (require 'tree-sitter-langs)
  (tree-sitter-require 'lumesh)
  (add-to-list 'tree-sitter-major-mode-language-alist '(lumesh-mode . lumesh)))

;; Lumesh mode
(define-derived-mode lumesh-mode shell-mode "Lumesh"
  "Major mode for Lumesh shell scripts."
  (setq font-lock-defaults '((lumesh-font-lock-keywords)))
  (tree-sitter-mode 1))

;; Auto-detect lumesh files
(add-to-list 'auto-mode-alist '("\\.lm\\'" . lumesh-mode))
(add-to-list 'auto-mode-alist '("\\.lumesh\\'" . lumesh-mode))
(add-to-list 'auto-mode-alist '("\\config\\.lm\\'" . lumesh-mode))
EOF

        log_success "Emacs configured with symlinks"
    else
        log_warning "Emacs not found, skipping..."
    fi
}

# Configure Sublime Text with symlinks
configure_sublime() {
    if [[ -d "$HOME/.config/sublime-text" ]] || [[ -d "$HOME/Library/Application Support/Sublime Text" ]]; then
        log_info "Configuring Sublime Text with symlinks..."

        # Detect Sublime Text directory
        if [[ -d "$HOME/.config/sublime-text" ]]; then
            SUBLIME_DIR="$HOME/.config/sublime-text"
        else
            SUBLIME_DIR="$HOME/Library/Application Support/Sublime Text"
        fi

        # Create package directory
        PACKAGE_DIR="$SUBLIME_DIR/Packages/User/Lumesh"
        mkdir -p "$PACKAGE_DIR"

        # Create symlink for WASM (Sublime doesn't use queries directly)
        create_symlink "$WASM_FILE" "$PACKAGE_DIR/lumesh.wasm"

        # Create syntax definition
        cat > "$PACKAGE_DIR/Lumesh.sublime-syntax" << 'EOF'
%YAML 1.2
---
name: Lumesh
file_extensions:
  - lm
  - lumesh
scope: source.lumesh

contexts:
  main:
    - include: comments
    - include: keywords
    - include: strings
    - include: numbers
    - include: operators

  comments:
    - match: '#.*$'
      scope: comment.line.number-sign.lumesh

  keywords:
    - match: '\b(let|fn|if|else|match|while|for|loop|return|break|continue|use|del|in|and|or|not)\b'
      scope: keyword.control.lumesh

  strings:
    - match: '"'
      scope: punctuation.definition.string.begin.lumesh
      push:
        - meta_scope: string.quoted.double.lumesh
        - match: '\\.'
          scope: constant.character.escape.lumesh
        - match: '"'
          scope: punctuation.definition.string.end.lumesh
          pop

  numbers:
    - match: '\b\d+\.?\d*\b'
      scope: constant.numeric.lumesh

  operators:
    - match: '(\+|\-|\*|\/|%|==|!=|<=?|>=?|=|&&|\|\||!|&|\||\^|~|<<|>>|\->|~>)'
      scope: keyword.operator.lumesh
EOF

        log_success "Sublime Text configured with symlinks"
    else
        log_warning "Sublime Text not found, skipping..."
    fi
}

# Verify symlinks are working
verify_installation() {
    log_info "Verifying installation..."

    local failed=0

    # Check central files exist
    if [[ ! -f "$WASM_FILE" ]]; then
        log_error "WASM file not found at $WASM_FILE"
        failed=1
    fi

    if [[ ! -d "$QUERIES_DIR" ]]; then
        log_error "Queries directory not found at $QUERIES_DIR"
        failed=1
    fi

    # Check some symlinks
    if command_exists nvim && [[ -L "$HOME/.config/nvim/lua/parser/lumesh.wasm" ]]; then
        if [[ ! -e "$HOME/.config/nvim/lua/parser/lumesh.wasm" ]]; then
            log_error "Neovim symlink is broken"
            failed=1
        fi
    fi

    if command_exists hx && [[ -L "$HOME/.local/share/helix/runtime/grammars/lumesh.wasm" ]]; then
        if [[ ! -e "$HOME/.local/share/helix/runtime/grammars/lumesh.wasm" ]]; then
            log_error "Helix symlink is broken"
            failed=1
        fi
    fi

    if [[ $failed -eq 0 ]]; then
        log_success "All symlinks verified successfully"
    else
        log_error "Some symlinks are broken. Please check the installation."
    fi
}

# Uninstall function to clean up symlinks
uninstall() {
    log_info "Uninstalling tree-sitter-lumesh..."

    # Remove central files
    if [[ -d "$INSTALL_DIR" ]]; then
        rm -rf "$INSTALL_DIR"
        log_success "Removed central installation directory"
    fi

    # Remove symlinks from editors
    local symlinks=(
        "$HOME/.config/nvim/lua/parser/lumesh.wasm"
        "$HOME/.config/nvim/after/queries/lumesh"
        "$HOME/.local/share/helix/runtime/grammars/lumesh.wasm"
        "$HOME/.local/share/helix/runtime/queries/lumesh"
        "$HOME/.config/zed/grammars/lumesh.wasm"
        "$HOME/.config/zed/grammars/lumesh-queries"
        "$HOME/.emacs.d/tree-sitter-langs/lumesh.wasm"
        "$HOME/.emacs.d/tree-sitter-langs/lumesh-queries"
    )

    for link in "${symlinks[@]}"; do
        if [[ -L "$link" ]]; then
            rm -f "$link"
            log_success "Removed symlink: $link"
        fi
    done

    # Remove VS Code extension directory
    if [[ -d "$HOME/.vscode/extensions/tree-sitter-lumesh-wasm" ]]; then
        rm -rf "$HOME/.vscode/extensions/tree-sitter-lumesh-wasm"
        log_success "Removed VS Code extension directory"
    fi

    log_success "Uninstallation complete"
}

# Main installation function
main() {
    echo "=== Tree-sitter Lumesh WASM Installation Script (Symlink Optimized) ==="
    echo "This script will download tree-sitter-lumesh WASM files and configure them for your editors using symlinks."
    echo ""

    # Parse command line arguments
    case "${1:-install}" in
        "install")
            ;;
        "uninstall")
            uninstall
            exit 0
            ;;
        "verify")
            verify_installation
            exit 0
            ;;
        *)
            echo "Usage: $0 [install|uninstall|verify]"
            exit 1
            ;;
    esac

    # Check dependencies
    if ! command_exists curl && ! command_exists wget; then
        log_error "curl or wget is required to download files"
        exit 1
    fi

    if ! command_exists tar; then
        log_error "tar command is required to extract queries"
        exit 1
    fi

    # Download files
    download_files

    # Configure editors
    echo ""
    log_info "Configuring editors with symlinks..."

    configure_neovim
    configure_vscode
    configure_helix
    # configure_zed
    configure_emacs
    configure_sublime

    # Verify installation
    echo ""
    verify_installation

    echo ""
    log_success "Installation complete!"
    echo ""
    echo "Central installation directory: $INSTALL_DIR"
    echo "All editors are using symlinks to this directory to save space."
    echo ""
    echo "To update: Re-run this script to download the latest version."
    echo "To uninstall: $0 uninstall"
    echo ""
    echo "Note: Some editors may require a restart to load the new grammar."
    echo "For VS Code, you may need to manually install the extension from:"
    echo "$HOME/.vscode/extensions/tree-sitter-lumesh-wasm"
}

# Run main function with all arguments
main "$@"

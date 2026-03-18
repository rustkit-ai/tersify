#!/usr/bin/env bash
# tersify installer
# Usage: curl -fsSL https://raw.githubusercontent.com/rustkit-ai/tersify/main/install.sh | bash
set -euo pipefail

REPO="rustkit-ai/tersify"
BINARY="tersify"
BIN_DIR="${TERSIFY_BIN_DIR:-$HOME/.local/bin}"

# ── Colour output ──────────────────────────────────────────────────────────────
red()   { printf "\033[31m%s\033[0m\n" "$*"; }
green() { printf "\033[32m%s\033[0m\n" "$*"; }
bold()  { printf "\033[1m%s\033[0m\n"  "$*"; }
info()  { printf "  %s\n" "$*"; }

# ── Detect OS + arch ──────────────────────────────────────────────────────────
detect_target() {
    local os arch
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-musl" ;;
                aarch64|arm64) echo "aarch64-unknown-linux-musl" ;;
                *) echo ""; return ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)  echo "x86_64-apple-darwin" ;;
                arm64)   echo "aarch64-apple-darwin" ;;
                *) echo ""; return ;;
            esac
            ;;
        *)
            echo ""
            ;;
    esac
}

# ── Fetch latest release tag ───────────────────────────────────────────────────
latest_tag() {
    if command -v curl &>/dev/null; then
        curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' \
            | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif command -v wget &>/dev/null; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' \
            | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    else
        echo ""
    fi
}

# ── Download + install binary ─────────────────────────────────────────────────
install_binary() {
    local target tag archive url tmpdir

    target=$(detect_target)
    if [[ -z "$target" ]]; then
        return 1
    fi

    tag=$(latest_tag)
    if [[ -z "$tag" ]]; then
        return 1
    fi

    archive="${BINARY}-${target}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${tag}/${archive}"

    info "Downloading ${BINARY} ${tag} for ${target}..."
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    if command -v curl &>/dev/null; then
        curl -fsSL "$url" -o "${tmpdir}/${archive}"
    elif command -v wget &>/dev/null; then
        wget -qO "${tmpdir}/${archive}" "$url"
    else
        red "Error: curl or wget is required."
        exit 1
    fi

    tar -xzf "${tmpdir}/${archive}" -C "${tmpdir}" "${BINARY}"
    mkdir -p "${BIN_DIR}"
    mv "${tmpdir}/${BINARY}" "${BIN_DIR}/${BINARY}"
    chmod +x "${BIN_DIR}/${BINARY}"

    green "✓ Installed ${BINARY} ${tag} → ${BIN_DIR}/${BINARY}"
    return 0
}

# ── Fallback: cargo install ───────────────────────────────────────────────────
install_via_cargo() {
    if ! command -v cargo &>/dev/null; then
        red "Error: cargo not found. Install Rust from https://rustup.rs"
        exit 1
    fi
    info "Building from source with cargo (this may take a minute)..."
    cargo install tersify --quiet
    green "✓ Installed ${BINARY} via cargo"
}

# ── PATH check ────────────────────────────────────────────────────────────────
ensure_path() {
    if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
        local shell_rc=""
        case "${SHELL:-}" in
            */zsh)  shell_rc="$HOME/.zshrc" ;;
            */bash) shell_rc="$HOME/.bashrc" ;;
            *)      shell_rc="$HOME/.profile" ;;
        esac

        echo "" >> "$shell_rc"
        echo "export PATH=\"\$PATH:${BIN_DIR}\"" >> "$shell_rc"

        printf "\n"
        bold "  Added ${BIN_DIR} to PATH in ${shell_rc}"
        info "  Reload your shell or run:  source ${shell_rc}"
        export PATH="${PATH}:${BIN_DIR}"
    fi
}

# ── Hook all editors ──────────────────────────────────────────────────────────
install_hooks() {
    printf "\n"
    bold "Installing editor hooks..."
    "${BIN_DIR}/${BINARY}" install --all 2>/dev/null \
        || tersify install --all 2>/dev/null \
        || true
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
    printf "\n"
    bold "tersify installer"
    printf "\n"

    # 1. Install the binary
    if ! install_binary; then
        info "Pre-built binary not available for this platform — falling back to cargo."
        install_via_cargo
    fi

    # 2. Ensure PATH is set
    ensure_path

    # 3. Hook into all detected AI editors
    install_hooks

    # 4. Final message
    printf "\n"
    green "✓ tersify is ready!"
    printf "\n"
    info "Check your savings anytime:  tersify stats"
    info "Compress a file:             tersify src/main.rs"
    info "AST mode (signatures only):  tersify src/ --ast"
    info "Uninstall hooks:             tersify uninstall --all"
    printf "\n"
}

main "$@"

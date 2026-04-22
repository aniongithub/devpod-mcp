#!/usr/bin/env bash
set -euo pipefail

# devcontainer-mcp installer
# Downloads the latest release binary.
# Backend CLIs (devpod, devcontainer, gh) are detected at runtime —
# if missing, the MCP server returns a helpful error message.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/aniongithub/devcontainer-mcp/main/install.sh | bash
#   curl -fsSL ... | bash -s -- --install-dir /usr/local/bin

REPO="aniongithub/devcontainer-mcp"
INSTALL_DIR="${HOME}/.local/bin"

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --install-dir)   INSTALL_DIR="$2"; shift 2 ;;
    --help|-h)
      echo "Usage: install.sh [--install-dir DIR]"
      echo "  --install-dir   Installation directory (default: ~/.local/bin)"
      exit 0
      ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# Detect OS and architecture
detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)  os="linux" ;;
    Darwin) os="darwin" ;;
    *)      echo "Error: Unsupported OS: $os"; exit 1 ;;
  esac

  case "$arch" in
    x86_64|amd64)  arch="x64" ;;
    aarch64|arm64) arch="arm64" ;;
    *)             echo "Error: Unsupported architecture: $arch"; exit 1 ;;
  esac

  echo "${os}-${arch}"
}

# Get latest release tag from GitHub
get_latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
}

PLATFORM="$(detect_platform)"
echo "==> Detected platform: ${PLATFORM}"

VERSION="$(get_latest_version)"
if [[ -z "$VERSION" ]]; then
  echo "Error: Could not determine latest release version."
  echo "Check: https://github.com/${REPO}/releases"
  exit 1
fi
echo "==> Latest version: ${VERSION}"

TARBALL_NAME="devcontainer-mcp-${PLATFORM}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${TARBALL_NAME}"

# Create install directory
mkdir -p "$INSTALL_DIR"

echo "==> Downloading ${TARBALL_NAME}..."
curl -fsSL "$DOWNLOAD_URL" | tar xz -C "${INSTALL_DIR}"
chmod +x "${INSTALL_DIR}/devcontainer-mcp"

# macOS: ad-hoc codesign to avoid Gatekeeper "Killed: 9"
if [[ "$(uname -s)" == "Darwin" ]]; then
  codesign -s - "${INSTALL_DIR}/devcontainer-mcp" 2>/dev/null && \
    echo "==> Codesigned binary for macOS" || true
fi

echo "==> Installed devcontainer-mcp to ${INSTALL_DIR}/devcontainer-mcp"

# Verify
if "${INSTALL_DIR}/devcontainer-mcp" --version >/dev/null 2>&1; then
  echo "==> $(${INSTALL_DIR}/devcontainer-mcp --version)"
else
  echo "Warning: Binary downloaded but failed to run. Check platform compatibility."
fi

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
  echo ""
  echo "Note: ${INSTALL_DIR} is not in your PATH. Add it with:"
  echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
fi

# Install SKILL.md for agent discovery
SKILL_URL="https://raw.githubusercontent.com/${REPO}/main/SKILL.md"
SKILL_DIRS=(
  "${HOME}/.copilot/skills/devcontainer-mcp"
  "${HOME}/.claude/skills/devcontainer-mcp"
  "${HOME}/.agents/skills/devcontainer-mcp"
)

echo ""
echo "==> Installing SKILL.md for agent discovery..."
for dir in "${SKILL_DIRS[@]}"; do
  mkdir -p "$dir"
  curl -fsSL -o "${dir}/SKILL.md" "$SKILL_URL" 2>/dev/null && \
    echo "    ${dir}/SKILL.md" || true
done

# Detect available backends
echo ""
echo "Backend CLIs detected (install as needed — MCP server gives helpful errors if missing):"
command -v devpod       >/dev/null 2>&1 && echo "  ✓ devpod"       || echo "  ✗ devpod        — https://devpod.sh/docs/getting-started/install"
command -v devcontainer >/dev/null 2>&1 && echo "  ✓ devcontainer"  || echo "  ✗ devcontainer   — npm install -g @devcontainers/cli"
command -v gh           >/dev/null 2>&1 && echo "  ✓ gh (codespaces)" || echo "  ✗ gh (codespaces) — https://cli.github.com/"

echo ""
echo "Done! Configure your MCP client:"
echo '  {'
echo '    "mcpServers": {'
echo '      "devcontainer-mcp": {'
echo "        \"command\": \"${INSTALL_DIR}/devcontainer-mcp\","
echo '        "args": ["serve"]'
echo '      }'
echo '    }'
echo '  }'

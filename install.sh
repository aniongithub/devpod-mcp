#!/usr/bin/env bash
set -euo pipefail

# devcontainer-mcp installer
# Downloads the latest release binary and installs DevPod CLI if not present.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/aniongithub/devcontainer-mcp/main/install.sh | bash
#   curl -fsSL ... | bash -s -- --install-dir /usr/local/bin
#   curl -fsSL ... | bash -s -- --skip-devpod

REPO="aniongithub/devcontainer-mcp"
INSTALL_DIR="${HOME}/.local/bin"
SKIP_DEVPOD=false

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-devpod)   SKIP_DEVPOD=true; shift ;;
    --install-dir)   INSTALL_DIR="$2"; shift 2 ;;
    --help|-h)
      echo "Usage: install.sh [--skip-devpod] [--install-dir DIR]"
      echo "  --skip-devpod   Skip automatic DevPod CLI installation"
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

BINARY_NAME="devcontainer-mcp-${PLATFORM}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY_NAME}"

# Create install directory
mkdir -p "$INSTALL_DIR"

echo "==> Downloading ${BINARY_NAME}..."
curl -fsSL -o "${INSTALL_DIR}/devcontainer-mcp" "$DOWNLOAD_URL"
chmod +x "${INSTALL_DIR}/devcontainer-mcp"
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

# Ensure DevPod CLI is available
if command -v devpod >/dev/null 2>&1; then
  echo ""
  echo "==> DevPod CLI already installed: $(devpod version)"
elif ! $SKIP_DEVPOD; then
  echo ""
  echo "==> DevPod CLI not found — installing..."
  DEVPOD_OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  DEVPOD_ARCH="$(uname -m)"

  case "$DEVPOD_ARCH" in
    x86_64|amd64)  DEVPOD_ARCH="amd64" ;;
    aarch64|arm64) DEVPOD_ARCH="arm64" ;;
  esac

  DEVPOD_URL="https://github.com/loft-sh/devpod/releases/latest/download/devpod-${DEVPOD_OS}-${DEVPOD_ARCH}"
  curl -fsSL -o "${INSTALL_DIR}/devpod" "$DEVPOD_URL"
  chmod +x "${INSTALL_DIR}/devpod"
  echo "==> Installed DevPod CLI to ${INSTALL_DIR}/devpod"
  echo "==> $(${INSTALL_DIR}/devpod version)"
else
  echo ""
  echo "Warning: DevPod CLI not found and --skip-devpod was set."
  echo "The MCP server requires DevPod to function. Install it from:"
  echo "  https://devpod.sh/docs/getting-started/install"
fi

# Install SKILL.md for agent discovery
SKILL_URL="https://raw.githubusercontent.com/${REPO}/main/SKILL.md"
SKILL_DIRS=(
  "${HOME}/.copilot/skills/devpod-mcp"
  "${HOME}/.claude/skills/devpod-mcp"
  "${HOME}/.agents/skills/devpod-mcp"
)

echo ""
echo "==> Installing SKILL.md for agent discovery..."
for dir in "${SKILL_DIRS[@]}"; do
  mkdir -p "$dir"
  curl -fsSL -o "${dir}/SKILL.md" "$SKILL_URL" 2>/dev/null && \
    echo "    ${dir}/SKILL.md" || true
done

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

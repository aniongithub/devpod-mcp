#!/usr/bin/env bash
# initialize.sh — runs on the HOST before the container starts.
# Grabs the GitHub token and writes it to gh.env for container use.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GH_ENV_FILE="${SCRIPT_DIR}/gh.env"

# --- Verify gh CLI ---
if ! command -v gh &>/dev/null; then
    echo "⚠️  gh CLI not found on host. Codespaces backend will not work."
    touch "${GH_ENV_FILE}"
    exit 0
fi

echo "🔐 Acquiring GitHub token for devcontainer..."

# Try to get token from gh CLI (works with keyring or GH_TOKEN)
GH_TOKEN=$(gh auth token -h github.com 2>/dev/null || true)
if [ -z "${GH_TOKEN}" ]; then
    echo "🔑 GitHub CLI login required..."
    gh auth login -h github.com -p https -w
    GH_TOKEN=$(gh auth token -h github.com 2>/dev/null || true)
fi

if [ -n "${GH_TOKEN}" ]; then
    echo "GH_TOKEN=${GH_TOKEN}" > "${GH_ENV_FILE}"
    echo "✅ GitHub token acquired"
else
    touch "${GH_ENV_FILE}"
    echo "⚠️  Could not acquire GitHub token — codespaces tools won't work"
fi

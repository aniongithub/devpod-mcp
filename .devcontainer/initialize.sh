#!/usr/bin/env bash
# initialize.sh — runs on the HOST before the container starts.
# Grabs the GitHub token and writes it to gh.env for container use.
# If multiple accounts are logged in, uses the active one.
# Set DEVCONTAINER_GH_USER=<login> to override.
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

# Get authenticated accounts from keyring (ignore GH_TOKEN env)
ACCOUNTS_JSON=$(GH_TOKEN="" gh auth status --json hosts 2>/dev/null || echo '{}')

# If user specified an account, use it
if [ -n "${DEVCONTAINER_GH_USER:-}" ]; then
    ACCOUNT="${DEVCONTAINER_GH_USER}"
    echo "Using specified GitHub account: ${ACCOUNT}"
else
    # Pick account: prefer active, fall back to first, prompt if interactive
    ACCOUNT=$(echo "$ACCOUNTS_JSON" | python3 -c "
import sys, json
data = json.load(sys.stdin)
hosts = data.get('hosts', {}).get('github.com', [])
ok = [h for h in hosts if h.get('state') == 'success']
active = [h for h in ok if h.get('active')]
pick = active[0] if active else (ok[0] if ok else None)
if pick:
    print(pick['login'])
" 2>/dev/null || true)

    ACCOUNT_COUNT=$(echo "$ACCOUNTS_JSON" | python3 -c "
import sys, json
data = json.load(sys.stdin)
hosts = data.get('hosts', {}).get('github.com', [])
print(len([h for h in hosts if h.get('state') == 'success']))
" 2>/dev/null || echo 0)

    if [ "$ACCOUNT_COUNT" -gt 1 ] && [ -t 0 ]; then
        # Interactive terminal — let user choose
        ALL_ACCOUNTS=$(echo "$ACCOUNTS_JSON" | python3 -c "
import sys, json
data = json.load(sys.stdin)
hosts = data.get('hosts', {}).get('github.com', [])
for h in hosts:
    if h.get('state') == 'success':
        marker = ' (active)' if h.get('active') else ''
        print(h['login'] + marker)
" 2>/dev/null || true)
        echo ""
        echo "Multiple GitHub accounts detected:"
        i=1
        while IFS= read -r acct; do
            echo "  ${i}) ${acct}"
            i=$((i + 1))
        done <<< "$ALL_ACCOUNTS"
        echo ""
        read -rp "Which account? [1-${ACCOUNT_COUNT}] (default: ${ACCOUNT}): " CHOICE
        if [ -n "$CHOICE" ]; then
            PICKED=$(echo "$ACCOUNTS_JSON" | python3 -c "
import sys, json
data = json.load(sys.stdin)
hosts = data.get('hosts', {}).get('github.com', [])
ok = [h['login'] for h in hosts if h.get('state') == 'success']
idx = int(sys.argv[1]) - 1
print(ok[idx] if 0 <= idx < len(ok) else '')
" "$CHOICE" 2>/dev/null || true)
            [ -n "$PICKED" ] && ACCOUNT="$PICKED"
        fi
    fi
fi

if [ -z "$ACCOUNT" ]; then
    echo "🔑 No GitHub accounts found. Logging in..."
    GH_TOKEN="" gh auth login -h github.com -p https -w
    GH_TOKEN=$(GH_TOKEN="" gh auth token -h github.com 2>/dev/null || true)
else
    echo "Using GitHub account: ${ACCOUNT}"
    GH_TOKEN=$(GH_TOKEN="" gh auth token -h github.com --user "${ACCOUNT}" 2>/dev/null || true)
fi

if [ -n "${GH_TOKEN:-}" ]; then
    echo "GH_TOKEN=${GH_TOKEN}" > "${GH_ENV_FILE}"
    echo "✅ GitHub token acquired (${ACCOUNT:-unknown})"
else
    touch "${GH_ENV_FILE}"
    echo "⚠️  Could not acquire GitHub token — codespaces tools won't work"
fi

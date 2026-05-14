#!/usr/bin/env bash
# devcontainer-guard.sh — PreToolUse hook for Claude Code & GitHub Copilot CLI
#
# Blocks bash/shell tool calls when .devcontainer/devcontainer.json exists in
# the working directory, forcing agents to use devcontainer-mcp MCP tools
# instead of running commands directly on the host.
#
# Read-only tools (view, grep, glob) and file edits are allowed through — only
# command execution is blocked.
#
# Host-safe commands (git, gh, curl, etc.) are allowlisted and always permitted
# since they operate on the repo/host, not the project's build environment.
#
# Bypass: include USER_CONFIRMED_HOST_OPERATION=1 in the command.
#
# Supports both agent payload formats:
#   Claude Code:  { tool_name, tool_input, cwd, ... }
#   Copilot CLI:  { toolName, toolArgs, cwd, ... }

set -euo pipefail

INPUT=$(cat)

# --- Detect agent format and extract fields ---

# Try Claude Code fields first (snake_case), fall back to Copilot CLI (camelCase)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // .toolName // empty')
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')

# Only guard bash/shell tool calls — allow everything else through
case "$TOOL_NAME" in
  Bash|bash|shell|powershell|Shell|PowerShell) ;;
  *) exit 0 ;;
esac

TOOL_INPUT=$(echo "$INPUT" | jq -r '(.tool_input // .toolArgs // {}) | tostring')

# Check for the bypass string anywhere in the tool input
if echo "$TOOL_INPUT" | grep -q 'USER_CONFIRMED_HOST_OPERATION=1'; then
  exit 0
fi

# Check if a devcontainer exists in the working directory
if [ -z "$CWD" ]; then
  # No cwd in payload — can't determine context, allow through
  exit 0
fi

if [ ! -f "${CWD}/.devcontainer/devcontainer.json" ]; then
  # No devcontainer — allow through
  exit 0
fi

# --- Devcontainer exists: check allowlist before blocking ---

# Extract the command string from tool input (handles both formats)
COMMAND=$(echo "$INPUT" | jq -r '(.tool_input.command // .toolArgs.command // "") | tostring')

# Commands that are safe to run on the host even when a devcontainer exists.
# These operate on the repo/host itself, not on the project's build environment.
ALLOWED_HOST_COMMANDS=(
  git
  gh
)

# Extract all meaningful commands from a shell string, skipping env vars
# (KEY=VALUE) and cd/pushd/popd. Splits on &&, ||, ;, and | to catch every
# command in a chain or pipeline.
all_commands() {
  local cmd="$1"
  while IFS= read -r segment; do
    segment="${segment#"${segment%%[![:space:]]*}"}"
    [ -z "$segment" ] && continue
    for token in $segment; do
      if [[ "$token" == *=* && "$token" != -* ]]; then
        continue
      fi
      case "$token" in
        cd|pushd|popd) break ;;
      esac
      basename "$token"
      break
    done
  done < <(echo "$cmd" | sed 's/ *&& */\n/g; s/ *|| */\n/g; s/ *; */\n/g; s/ *| */\n/g')
}

# Every command in the chain must be on the allowlist
ALL_ALLOWED=true
while IFS= read -r cmd_name; do
  [ -z "$cmd_name" ] && continue
  FOUND=false
  for allowed in "${ALLOWED_HOST_COMMANDS[@]}"; do
    if [ "$cmd_name" = "$allowed" ]; then
      FOUND=true
      break
    fi
  done
  if [ "$FOUND" = false ]; then
    ALL_ALLOWED=false
    break
  fi
done < <(all_commands "$COMMAND")

if [ "$ALL_ALLOWED" = true ] && [ -n "$(all_commands "$COMMAND")" ]; then
  exit 0
fi

# --- Not on the allowlist: block the tool call ---

DENY_REASON="Host execution blocked. This project has a devcontainer. Use devcontainer-mcp tools (devcontainer_exec, devpod_ssh, codespaces_ssh, and file operation tools) instead of running commands directly on the host."

# Detect which agent format to use for the response
if echo "$INPUT" | jq -e '.tool_name // empty' >/dev/null 2>&1 && \
   [ -n "$(echo "$INPUT" | jq -r '.tool_name // empty')" ]; then
  # Claude Code format
  jq -n --arg reason "$DENY_REASON" '{
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "deny",
      permissionDecisionReason: $reason
    }
  }'
else
  # Copilot CLI format
  jq -n --arg reason "$DENY_REASON" '{
    permissionDecision: "deny",
    permissionDecisionReason: $reason
  }'
fi

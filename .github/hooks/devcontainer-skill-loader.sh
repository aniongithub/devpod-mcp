#!/usr/bin/env bash
# devcontainer-skill-loader.sh — SessionStart hook for Claude Code & Copilot CLI
#
# When a session starts in a directory with .devcontainer/devcontainer.json,
# injects the devcontainer-mcp SKILL.md content as additionalContext so the
# agent automatically knows how to use devcontainer-mcp tools.
#
# Supports both agent payload formats:
#   Claude Code:  { tool_name, tool_input, cwd, ... }
#   Copilot CLI:  { toolName, toolArgs, cwd, ... }

set -euo pipefail

INPUT=$(cat)

# Extract working directory from the payload
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')

if [ -z "$CWD" ]; then
  exit 0
fi

if [ ! -f "${CWD}/.devcontainer/devcontainer.json" ]; then
  exit 0
fi

# Look for SKILL.md in order of preference
SKILL_PATH=""
SEARCH_PATHS=(
  "${HOME}/.local/share/devcontainer-mcp/SKILL.md"
  "${HOME}/.copilot/skills/devcontainer-mcp/SKILL.md"
  "${HOME}/.claude/skills/devcontainer-mcp/SKILL.md"
  "${HOME}/.agents/skills/devcontainer-mcp/SKILL.md"
)

for p in "${SEARCH_PATHS[@]}"; do
  if [ -f "$p" ]; then
    SKILL_PATH="$p"
    break
  fi
done

if [ -z "$SKILL_PATH" ]; then
  exit 0
fi

SKILL_CONTENT=$(cat "$SKILL_PATH")

jq -n --arg ctx "$SKILL_CONTENT" '{ "additionalContext": $ctx }'

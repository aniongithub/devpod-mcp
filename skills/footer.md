---
tags: [core]
order: 90
---
## What NOT to do

- ❌ Do NOT install packages on the host
- ❌ Do NOT run builds on the host
- ❌ Do NOT modify the host's global config
- ❌ Do NOT run `docker`, `devcontainer`, `devpod`, or `gh` CLI commands directly — use the MCP tools
- ✅ DO authenticate before using codespaces tools
- ✅ DO ask the user which account/machine type to use
- ✅ DO use `devpod_ssh`, `devcontainer_exec`, or `codespaces_ssh` for everything
- ✅ DO check `.devcontainer/devcontainer.json` first

> **Note:** Host-protection hooks are installed for supported agent environments (Claude Code, GitHub Copilot CLI) that automatically block shell commands when a devcontainer is detected. If a command is blocked, use the appropriate MCP tool instead.

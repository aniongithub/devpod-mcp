---
tags: [core]
order: 20
---
## Core Rules

**If a project has `.devcontainer/devcontainer.json`, ALL work MUST happen inside a dev container — never install dependencies, run builds, or execute code directly on the host.**

**Use ONLY the MCP tools listed here.** Do not invoke `docker`, `devcontainer`, `devpod`, `gh`, or `wsl` CLI commands directly — the MCP tools wrap these CLIs with proper error handling, auth resolution, and escaping. Direct CLI usage bypasses these safeguards. This applies even when the user asks to work "directly in WSL" or "not in a devcontainer" — use `wsl_exec` and WSL file tools instead of raw `wsl` commands.

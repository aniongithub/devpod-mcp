---
tags: [core]
order: 20
---
## Core Rules

**If a project has `.devcontainer/devcontainer.json`, ALL work MUST happen inside a dev container — never install dependencies, run builds, or execute code directly on the host.**

**Use ONLY the MCP tools listed here.** Do not invoke `docker`, `devcontainer`, `devpod`, or `gh` CLI commands directly — the MCP tools wrap these CLIs with proper error handling, auth resolution, and escaping. Direct CLI usage bypasses these safeguards.

---
name: devcontainer-mcp
description: Manage dev container environments via MCP tools (DevPod, devcontainer CLI, Codespaces)
tools:
  - devpod_up
  - devpod_stop
  - devpod_delete
  - devpod_build
  - devpod_status
  - devpod_list
  - devpod_ssh
  - devpod_logs
  - devpod_provider_list
  - devpod_provider_add
  - devpod_provider_delete
  - devpod_context_list
  - devpod_context_use
  - devpod_container_inspect
  - devpod_container_logs
  - devcontainer_up
  - devcontainer_exec
  - devcontainer_build
  - devcontainer_read_config
  - devcontainer_stop
  - devcontainer_remove
  - devcontainer_status
  - codespaces_create
  - codespaces_list
  - codespaces_ssh
  - codespaces_stop
  - codespaces_delete
  - codespaces_view
  - codespaces_ports
---

# DevContainer MCP Skill

You have access to `devcontainer-mcp`, an MCP server that manages dev container environments across three backends:
- **DevPod** (`devpod_*` tools) — multi-provider: Docker, K8s, AWS, GCP, etc.
- **devcontainer CLI** (`devcontainer_*` tools) — local Docker via the official CLI
- **GitHub Codespaces** (`codespaces_*` tools) — cloud-hosted environments

## Core Rule

**If a project has `.devcontainer/devcontainer.json`, ALL work MUST happen inside a dev container — never install dependencies, run builds, or execute code directly on the host.**

## Choosing a Backend

1. **Local Docker + devcontainer CLI** — simplest for local development
2. **DevPod** — when you need multi-provider support or the project uses DevPod
3. **Codespaces** — when you need cloud-hosted environments or the user prefers GitHub infrastructure

## Workflow: DevPod

### 1. Create or start the workspace
```
devpod_up(args: "/path/to/project --id my-project --provider docker")
```

### 2. Verify the workspace is running
```
devpod_status(workspace: "my-project")
```

### 3. Execute commands inside the workspace
```
devpod_ssh(workspace: "my-project", command: "cargo build")
```

### 4. Stop when done
```
devpod_stop(workspace: "my-project")
```

## Workflow: devcontainer CLI

### 1. Start the dev container
```
devcontainer_up(workspace_folder: "/path/to/project")
```

### 2. Execute commands
```
devcontainer_exec(workspace_folder: "/path/to/project", command: "npm test")
```

### 3. Stop when done
```
devcontainer_stop(workspace_folder: "/path/to/project")
```

## Workflow: Codespaces

### 1. Create a codespace
```
codespaces_create(repo: "owner/repo", machine: "basicLinux")
```

### 2. Execute commands
```
codespaces_ssh(codespace: "codespace-name", command: "npm test")
```

### 3. Stop when done
```
codespaces_stop(codespace: "codespace-name")
```

## Self-Healing

If `devpod_up`, `devcontainer_up`, or `codespaces_create` returns errors:
1. Read the error output carefully
2. Fix the `Dockerfile` or `devcontainer.json`
3. Call the up/create command again
4. Repeat until successful

## What NOT to do

- ❌ Do NOT install packages on the host
- ❌ Do NOT run builds on the host
- ❌ Do NOT modify the host's global config
- ✅ DO use `devpod_ssh`, `devcontainer_exec`, or `codespaces_ssh` for everything
- ✅ DO check `.devcontainer/devcontainer.json` first

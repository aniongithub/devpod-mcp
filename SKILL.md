---
name: devpod-mcp
description: Manage isolated DevPod development environments via MCP tools
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
---

# DevPod MCP Skill

You have access to `devpod-mcp`, an MCP server that manages isolated development environments via [DevPod](https://devpod.sh/).

## Core Rule

**If a project has `.devcontainer/devcontainer.json`, ALL work MUST happen inside a DevPod workspace — never install dependencies, run builds, or execute code directly on the host.**

## Workflow

### 1. Check for devcontainer config
Before doing any work on a project, check if `.devcontainer/devcontainer.json` exists.

### 2. Create or start the workspace
```
devpod_up(args: "/path/to/project --id my-project --provider docker")
```
- Always use `--provider docker` unless the user specifies otherwise.
- The workspace ID should be a short, descriptive kebab-case name.

### 3. Verify the workspace is running
```
devpod_status(workspace: "my-project")
```
Expect `"state": "Running"`.

### 4. Execute all commands inside the workspace
```
devpod_ssh(workspace: "my-project", command: "cargo build")
devpod_ssh(workspace: "my-project", command: "npm install && npm test")
```
- Use `devpod_ssh` for ALL commands: builds, tests, linting, package installs, etc.
- Never run these on the host.

### 5. Handle build failures (self-healing)
If `devpod_up` returns errors in stderr:
1. Read the error output carefully
2. Fix the `Dockerfile` or `devcontainer.json` in the project
3. Call `devpod_up` again — DevPod will rebuild with the fix
4. Repeat until successful

If `devpod_ssh` commands fail, check `devpod_logs` for container-level issues.

### 6. Stop when done
```
devpod_stop(workspace: "my-project")
```

## What NOT to do

- ❌ Do NOT install packages on the host (npm install, pip install, apt install, etc.)
- ❌ Do NOT run builds on the host
- ❌ Do NOT modify the host's global config (PATH, env vars, etc.)
- ❌ Do NOT assume host tools match what the project needs
- ✅ DO use `devpod_ssh` for everything
- ✅ DO check `.devcontainer/devcontainer.json` first
- ✅ DO return build errors to the user for devcontainer config issues

## Inspecting containers

Use `devpod_container_inspect` and `devpod_container_logs` when you need low-level Docker details (ports, labels, mounts, raw container logs) that `devpod_status` and `devpod_logs` don't cover.

## Managing multiple workspaces

Use `devpod_list` to see all workspaces. Each workspace is independent — you can run multiple projects simultaneously in separate containers.

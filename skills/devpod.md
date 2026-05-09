---
tags: [core]
order: 50
---
## Workflow: DevPod

> **Use these tools — not raw `devpod` CLI commands.**

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

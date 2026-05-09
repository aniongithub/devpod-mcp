---
tags: [wsl]
order: 75
---
## Workflow: WSL (Windows only)

> **Use these tools — not raw `wsl.exe` or PowerShell `wsl` commands.** When a user asks to work "in WSL" or "directly in WSL", use `wsl_exec` and the WSL file tools below — these ARE the way to work in WSL through MCP.

WSL tools let you clone repos, build projects, and run commands inside any WSL distribution — without devcontainers or Docker.

### 1. List available distributions
```
wsl_list()
```

### 2. Clone and build a repo in WSL
```
wsl_exec(distro: "Ubuntu", command: "git clone https://github.com/org/repo.git /home/user/repo")
wsl_exec(distro: "Ubuntu", command: "cd /home/user/repo && cargo build")
```

### 3. Execute any command inside a distribution
```
wsl_exec(distro: "Ubuntu", command: "apt list --installed")
```

### 4. Set the default distribution
```
wsl_set_default(distro: "Ubuntu")
```

### 5. Stop a distribution
```
wsl_terminate(distro: "Ubuntu")
```

### 6. Shut down all WSL distributions
```
wsl_shutdown()
```

### File operations in WSL
```
wsl_file_read(distro: "Ubuntu", path: "/home/user/project/src/main.rs")
wsl_file_write(distro: "Ubuntu", path: "/home/user/file.txt", content: "fn main() {}")
wsl_file_edit(distro: "Ubuntu", path: "/home/user/file.txt", old_str: "fn main", new_str: "fn start")
wsl_file_list(distro: "Ubuntu", path: "/home/user/project")
```

---
tags: [wsl]
order: 75
---
## Workflow: WSL (Windows only)

WSL tools are available when running on Windows. They let you manage and interact with WSL distributions directly.

### 1. List available distributions
```
wsl_list()
```

### 2. Execute commands inside a distribution
```
wsl_exec(distro: "Ubuntu", command: "apt list --installed")
```

### 3. Set the default distribution
```
wsl_set_default(distro: "Ubuntu")
```

### 4. Stop a distribution
```
wsl_terminate(distro: "Ubuntu")
```

### 5. Shut down all WSL distributions
```
wsl_shutdown()
```

### File operations in WSL
```
wsl_file_read(distro: "Ubuntu", path: "/home/user/project/src/main.rs")
wsl_file_write(distro: "Ubuntu", path: "/home/user/file.txt", content: "hello")
wsl_file_edit(distro: "Ubuntu", path: "/home/user/file.txt", old_str: "hello", new_str: "world")
wsl_file_list(distro: "Ubuntu", path: "/home/user/project")
```


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

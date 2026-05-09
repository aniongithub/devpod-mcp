---
tags: [core]
order: 70
---
## Workflow: Codespaces

### 1. Authenticate
```
auth_status(provider: "github")
# If no accounts: auth_login(provider: "github", scopes: "codespace")
# If multiple: ask the user which account
```

### 2. Create a codespace — ask user for machine type
```
codespaces_create(auth: "github-USERNAME", repo: "owner/repo", machine: "basicLinux32gb")
```
Machine types: `basicLinux32gb` (2 cores, 8 GB), `standardLinux32gb` (4 cores, 16 GB), `premiumLinux` (8 cores, 32 GB), `largePremiumLinux` (16 cores, 64 GB)

### 3. Execute commands
```
codespaces_ssh(auth: "github-USERNAME", codespace: "codespace-name", command: "npm test")
```

### 4. Stop when done
```
codespaces_stop(auth: "github-USERNAME", codespace: "codespace-name")
```

---
tags: [core]
order: 30
---
## Authentication

**Before using Codespaces tools, you MUST obtain an auth handle.**

### 1. Check available accounts
```
auth_status(provider: "github")
→ returns accounts with IDs like "github-aniongithub"
```

### 2. If multiple accounts, ask the user which one to use

### 3. If no accounts or missing scopes, initiate login
```
auth_login(provider: "github", scopes: "codespace")
→ opens browser, copies device code to clipboard
→ tell the user: "Approve in the browser, code is on your clipboard"
```

### 4. Pass the auth handle to all codespaces_* tools
```
codespaces_create(auth: "github-aniongithub", repo: "owner/repo", ...)
```

**The agent never sees raw tokens.** Auth handles are opaque IDs resolved by the MCP server.

Supported auth providers: `github`, `aws`, `azure`, `gcloud`, `kubernetes`

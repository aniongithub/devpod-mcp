---
tags: [core]
order: 80
---
## Self-Healing

If `devpod_up`, `devcontainer_up`, or `codespaces_create` returns errors:
1. Read the error output carefully
2. Fix the `Dockerfile` or `devcontainer.json`
3. Call the up/create command again
4. Repeat until successful

---
tags: [core]
order: 90
---
## What NOT to do

- ❌ Do NOT install packages on the host
- ❌ Do NOT run builds on the host
- ❌ Do NOT modify the host's global config
- ✅ DO authenticate before using codespaces tools
- ✅ DO ask the user which account/machine type to use
- ✅ DO use `devpod_ssh`, `devcontainer_exec`, or `codespaces_ssh` for everything
- ✅ DO check `.devcontainer/devcontainer.json` first

## File Operations

**All backends support built-in file operations — no need to construct shell commands.**

These tools mirror familiar editing tools (read, write, edit, list) and handle escaping, encoding, and directory creation automatically.

### Reading files
```
devpod_file_read(workspace: "my-ws", path: "/workspaces/project/src/main.rs")
devcontainer_file_read(workspace_folder: "/path/to/project", path: "/workspaces/project/src/main.rs")
codespaces_file_read(auth: "github-user", codespace: "name", path: "src/main.rs")
```
Supports optional `start_line` and `end_line` for reading specific ranges.

### Writing files
```
devpod_file_write(workspace: "my-ws", path: "/workspaces/project/new_file.rs", content: "fn main() {}")
devcontainer_file_write(workspace_folder: "/path/to/project", path: "new_file.rs", content: "fn main() {}")
codespaces_file_write(auth: "github-user", codespace: "name", path: "src/new.rs", content: "...")
```
Creates parent directories automatically.

### Editing files (surgical replacement)
```
devpod_file_edit(workspace: "my-ws", path: "src/main.rs", old_str: "fn old()", new_str: "fn new()")
devcontainer_file_edit(workspace_folder: "/path/to/project", path: "src/lib.rs", old_str: "v1", new_str: "v2")
codespaces_file_edit(auth: "github-user", codespace: "name", path: "src/lib.rs", old_str: "TODO", new_str: "DONE")
```
`old_str` must match exactly once in the file. Include surrounding context to make it unique.

### Listing directories
```
devpod_file_list(workspace: "my-ws", path: "/workspaces/project/src")
devcontainer_file_list(workspace_folder: "/path/to/project", path: "src")
codespaces_file_list(auth: "github-user", codespace: "name", path: ".")
```
Shows non-hidden files up to 2 levels deep.

### When to use file tools vs exec/ssh
- ✅ **Use file tools** for reading, writing, and editing source files
- ✅ **Use exec/ssh** for running builds, tests, and commands
- ❌ **Don't** construct `sed`, `cat`, or `echo` commands via exec for file editing

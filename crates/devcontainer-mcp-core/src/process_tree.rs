//! Process-tree reaping.
//!
//! When a CLI we spawned crosses a process boundary (e.g. `devcontainer
//! exec` → `docker exec` → containerd-shim → user command inside the
//! container), simply SIGTERM'ing our immediate child does not kill the
//! actual work — the in-container processes are reparented to the
//! container's PID 1 and keep running. They consume CPU, hold locks,
//! and never exit.
//!
//! On Linux, every process in any namespace still has a host-visible PID
//! in the root PID namespace, and `/proc/<pid>/status` exposes its parent
//! as `PPid:`. So from our root child PID we can BFS the full set of
//! descendants and terminate them ourselves — no in-container shim, no
//! cooperation from the backend CLI required.
//!
//! Non-Linux platforms get a no-op fallback that only signals the root
//! PID; this is fine because dev containers are a Linux-host feature.

use std::time::Duration;

/// Reap a process tree rooted at `root_pid`.
///
/// 1. Discover every descendant of `root_pid` (including `root_pid` itself)
///    by walking `/proc/*/status`.
/// 2. Send `SIGTERM` to all of them.
/// 3. Wait up to `grace` for them to exit.
/// 4. Send `SIGKILL` to any survivors.
///
/// Best-effort: any individual `/proc` read or `kill(2)` is allowed to
/// fail silently (the process may have already exited). The function
/// returns when the grace period is up; it does not guarantee every PID
/// is gone (a process can ignore SIGKILL only if it is uninterruptible
/// in the kernel, which is out of our control).
pub async fn reap(root_pid: u32, grace: Duration) {
    #[cfg(target_os = "linux")]
    {
        reap_linux(root_pid, grace).await;
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Fallback: signal the root only. Container reaping requires
        // /proc-style introspection which we don't have here.
        let _ = (root_pid, grace);
        #[cfg(unix)]
        unsafe {
            libc::kill(root_pid as i32, libc::SIGTERM);
            tokio::time::sleep(grace).await;
            libc::kill(root_pid as i32, libc::SIGKILL);
        }
    }
}

#[cfg(target_os = "linux")]
async fn reap_linux(root_pid: u32, grace: Duration) {
    // Snapshot the full descendant set. We do this once, send SIGTERM,
    // wait, then re-discover for SIGKILL (new grandchildren may have
    // appeared during the grace period — e.g. a `go test` runner that
    // spawned its own test binaries while we were waiting).
    let initial = collect_descendants(root_pid);
    tracing::debug!(
        root = root_pid,
        n = initial.len(),
        "reap: SIGTERM descendants"
    );
    signal_all(&initial, libc_signal::SIGTERM);

    // Wait for graceful exit. Short-circuit if everyone is already gone.
    let deadline = std::time::Instant::now() + grace;
    while std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if !any_alive(&initial) {
            tracing::debug!(root = root_pid, "reap: all descendants exited cleanly");
            return;
        }
    }

    // Re-discover and SIGKILL anything still around. We re-walk /proc
    // because the surviving tree may have grown.
    let survivors = collect_descendants(root_pid);
    if !survivors.is_empty() {
        tracing::warn!(
            root = root_pid,
            n = survivors.len(),
            "reap: SIGKILL survivors"
        );
        signal_all(&survivors, libc_signal::SIGKILL);
    }
}

#[cfg(target_os = "linux")]
fn collect_descendants(root: u32) -> Vec<u32> {
    // Build a parent → children map by scanning /proc/*/status. This is
    // O(N) where N is the number of processes on the host. For typical
    // dev machines (a few hundred procs) this is sub-millisecond.
    use std::collections::HashMap;

    let mut children_of: HashMap<u32, Vec<u32>> = HashMap::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return vec![root];
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else { continue };
        let Ok(pid) = name_str.parse::<u32>() else { continue };
        if let Some(ppid) = read_ppid(pid) {
            children_of.entry(ppid).or_default().push(pid);
        }
    }

    // BFS from `root`. Include `root` itself so the caller can kill the
    // whole tree in one pass.
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(pid) = stack.pop() {
        out.push(pid);
        if let Some(kids) = children_of.get(&pid) {
            stack.extend_from_slice(kids);
        }
    }
    out
}

#[cfg(target_os = "linux")]
fn read_ppid(pid: u32) -> Option<u32> {
    // `/proc/<pid>/status` is line-oriented. We only need the `PPid:`
    // line, which appears near the top, so reading the whole file is
    // wasteful but trivial in size.
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("PPid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn signal_all(pids: &[u32], sig: i32) {
    for &pid in pids {
        // kill(2) can fail with ESRCH if the process already exited;
        // EPERM if we don't own it (shouldn't happen for our descendants
        // unless they did a setuid). Either way, swallow.
        unsafe {
            libc::kill(pid as i32, sig);
        }
    }
}

#[cfg(target_os = "linux")]
fn any_alive(pids: &[u32]) -> bool {
    pids.iter().any(|&pid| {
        // kill(pid, 0) probes existence + permission without sending a
        // signal. Returns 0 if the process exists and we can signal it.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    })
}

#[cfg(target_os = "linux")]
mod libc_signal {
    pub const SIGTERM: i32 = libc::SIGTERM;
    pub const SIGKILL: i32 = libc::SIGKILL;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn read_ppid_for_self() {
        let pid = std::process::id();
        let ppid = read_ppid(pid).expect("self has a PPid");
        // Whatever launched the test runner.
        assert!(ppid > 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn descendants_includes_root() {
        let pid = std::process::id();
        let set = collect_descendants(pid);
        assert!(set.contains(&pid));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn reap_kills_a_child_tree() {
        // Spawn `sh -c 'sleep 30 & sleep 30 & wait'` and reap it.
        let mut child = tokio::process::Command::new("sh")
            .args(["-c", "sleep 30 & sleep 30 & wait"])
            .kill_on_drop(true)
            .spawn()
            .expect("spawn");
        let root = child.id().expect("pid");
        // Let the grandchildren actually exist before we reap.
        tokio::time::sleep(Duration::from_millis(200)).await;
        let before = collect_descendants(root);
        assert!(before.len() >= 3, "expected root + 2 sleeps, got {before:?}");

        reap(root, Duration::from_secs(1)).await;
        // Give the kernel a moment to actually reap zombies.
        let _ = child.wait().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!any_alive(&before), "some descendants survived: {before:?}");
    }
}

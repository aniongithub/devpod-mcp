//! In-target process-group shim for cancellable cross-boundary execs.
//!
//! When we run a command across a process boundary (host → container via
//! `docker exec`, host → VM via SSH, …) we lose direct ancestry control:
//! the target-side processes do not appear as descendants of any host PID
//! we own. SIGTERM'ing our host-side wrapper closes the stdio pipes but
//! leaves the actual workload running, reparented to the target's init.
//!
//! To regain control we wrap the user's command with a tiny shim that:
//!   1. Runs the user command in a fresh session (`setsid`), so the
//!      leaf shell's PID is also its process group ID (PGID), and the
//!      PGID transitively addresses every descendant.
//!   2. Emits a recognizable sentinel on stderr containing that PGID,
//!      so the host-side reader can capture it.
//!
//! On cancellation, the host sends `kill -- -<pgid>` *inside the target*
//! (via the backend's exec channel — `docker exec`, `devpod ssh`, etc.),
//! which reaps the entire process group atomically.

/// Marker that brackets the PGID line so we can find and strip it from
/// stderr without false positives from user output.
const SENTINEL_PREFIX: &str = "__DCMCP_PGID=";
const SENTINEL_SUFFIX: &str = "__";

/// Wrap a user-supplied shell command with the shim prelude.
///
/// The returned string is intended to be passed as a single argument to
/// `sh -c` (or any POSIX shell) on the target. It expects to find the
/// user's command in the environment variable [`USER_CMD_ENV`] (see
/// [`USER_CMD_ENV`] for the name) — passing it that way avoids the
/// quoting hazards of nesting `sh -c '...'` inside another `sh -c '...'`
/// when the user command itself contains single quotes, parentheses, or
/// other shell metacharacters.
///
/// The shim runs the user command via `eval`, in a fresh session
/// (`setsid`) so the leaf shell's PID is also its process group ID
/// (PGID), and prints `__DCMCP_PGID=<pid>__` to stderr once for the
/// host-side reader to capture.
pub fn wrap() -> String {
    // We deliberately keep this string free of any user-supplied
    // content. Every variable expansion comes from the target shell's
    // environment, not from string interpolation on the host.
    //
    // We background the `setsid` child and `wait` on it instead of
    // using `setsid -w` because the `-w` flag is util-linux–only:
    // busybox/alpine images ship a `setsid` without it. Backgrounding
    // + `wait` is POSIX and works on every shell we care about.
    //
    // The trailing `exit "$?"` propagates the inner program's exit
    // code back up to the outer `sh -c` so `devcontainer exec`
    // returns the correct status.
    format!(
        r#"if command -v setsid >/dev/null 2>&1; then
  setsid sh -c 'printf "{prefix}%s{suffix}\n" "$$" 1>&2; eval "${env}"' &
  __dcmcp_pid=$!
  wait "$__dcmcp_pid"
  exit "$?"
else
  printf "{prefix}%s{suffix}\n" "$$" 1>&2
  eval "${env}"
fi"#,
        prefix = SENTINEL_PREFIX,
        suffix = SENTINEL_SUFFIX,
        env = USER_CMD_ENV,
    )
}

/// Environment variable through which the wrapped shim receives the
/// user command. Callers must set this in the spawned process's env
/// before invoking the shim string.
pub const USER_CMD_ENV: &str = "DCMCP_USER_CMD";

/// If `line` is the shim's sentinel, return the captured PGID.
/// Otherwise return `None`. The caller should suppress matching lines
/// from any downstream output.
pub fn try_parse_sentinel(line: &str) -> Option<i32> {
    // Match `^\s*__DCMCP_PGID=(\d+)__\s*$`. A hand-rolled scan keeps
    // the dependency footprint at zero.
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix(SENTINEL_PREFIX)?;
    let num = rest.strip_suffix(SENTINEL_SUFFIX)?;
    if num.is_empty() || !num.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    num.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_emits_sentinel_and_runs_user_cmd() {
        // Smoke test: pass user cmd via env, confirm stdout has the
        // user output and stderr has the sentinel.
        let wrapped = wrap();
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&wrapped)
            .env(USER_CMD_ENV, "echo hello")
            .output()
            .expect("spawn sh");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stdout.contains("hello"), "stdout: {stdout:?}");
        assert!(
            stderr.lines().any(|l| try_parse_sentinel(l).is_some()),
            "no sentinel in stderr: {stderr:?}"
        );
    }

    #[test]
    fn wrap_handles_metacharacters_in_user_cmd() {
        // The whole reason wrap() reads from env: user commands with
        // single quotes, parens, subshells, etc. must not break the
        // shim's own quoting.
        let wrapped = wrap();
        let evil = r#"(echo "it's a (test)" & echo $((1+2)) ) | tr a-z A-Z"#;
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&wrapped)
            .env(USER_CMD_ENV, evil)
            .output()
            .expect("spawn sh");
        assert!(out.status.success(), "stderr: {:?}", String::from_utf8_lossy(&out.stderr));
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("IT'S A (TEST)"), "stdout: {stdout:?}");
        assert!(stdout.contains("3"), "stdout missing arithmetic: {stdout:?}");
    }

    #[test]
    fn try_parse_sentinel_matches() {
        assert_eq!(try_parse_sentinel("__DCMCP_PGID=42__"), Some(42));
        assert_eq!(try_parse_sentinel("  __DCMCP_PGID=12345__  "), Some(12345));
    }

    #[test]
    fn try_parse_sentinel_rejects_garbage() {
        assert_eq!(try_parse_sentinel("hello"), None);
        assert_eq!(try_parse_sentinel("__DCMCP_PGID=abc__"), None);
        assert_eq!(try_parse_sentinel("PGID=42"), None);
        // Don't accept it embedded in other text:
        assert_eq!(try_parse_sentinel("prefix __DCMCP_PGID=42__"), None);
    }

    #[test]
    fn sentinel_pgid_actually_addresses_descendants() {
        // Run a sleep in the background under the shim and verify the
        // emitted PGID > 0 (round-trip parse). We can't safely kill
        // the group here without affecting the test runner, so the
        // end-to-end kill is tested by the cli integration test.
        let wrapped = wrap();
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&wrapped)
            .env(USER_CMD_ENV, "sleep 0.2 & wait")
            .output()
            .expect("spawn");
        let stderr = String::from_utf8_lossy(&out.stderr);
        let pgid = stderr
            .lines()
            .find_map(try_parse_sentinel)
            .expect("sentinel emitted");
        assert!(pgid > 0);
    }
}

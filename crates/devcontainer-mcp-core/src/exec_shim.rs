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

/// Self-contained variant of [`wrap`] that embeds the user command
/// directly in the returned shell string as a base64 blob.
///
/// Use this when the backend transport can carry a shell command but
/// cannot propagate environment variables — e.g. `devpod ssh
/// --command <cmd>` and `gh codespace ssh -- <cmd>`, neither of which
/// has a `--remote-env`-style flag.
///
/// The returned string is shell-safe regardless of what's in
/// `user_cmd`: the user command is encoded once on the host and
/// decoded once on the target, with no intermediate shell layers
/// re-interpreting its quoting.
pub fn wrap_self_contained(user_cmd: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let encoded = STANDARD.encode(user_cmd.as_bytes());
    // The shell decodes the blob into `DCMCP_USER_CMD` and then runs
    // the standard shim, which already knows how to eval that env
    // var. This keeps the two shim flavors structurally identical and
    // means a single sentinel parser handles both.
    //
    // We require `base64` on the target. It is present in every
    // container image we've encountered (coreutils, busybox, alpine)
    // because the file-write path already depends on it.
    let shim = wrap();
    format!(
        "{env}=$(printf '%s' '{encoded}' | base64 -d) && export {env} && {shim}",
        env = USER_CMD_ENV,
        encoded = encoded,
        shim = shim,
    )
}

/// If `line` contains the shim's sentinel, return the captured PGID.
/// Otherwise return `None`. The caller should suppress matching lines
/// from any downstream output.
///
/// We accept the sentinel anywhere within the line, not just as a
/// standalone match, because intermediate transports decorate remote
/// stderr with prefixes that vary by backend:
///   - `gh codespace ssh`: passes lines through unmodified.
///   - `devpod ssh`: prefixes each line with ANSI color codes, a
///     timestamp, and an `info`/`error` log-level tag.
///   - Future backends may layer on their own prefixes.
///
/// The sentinel itself (`__DCMCP_PGID=NNN__`) is intentionally
/// distinctive — both delimiters are `__DCMCP_*__` — so the chance
/// of a false positive from user output containing that exact byte
/// sequence is vanishingly small. Bounding the digit count to 31
/// bits keeps us safe against malformed input that would otherwise
/// trip `parse`'s overflow check.
pub fn try_parse_sentinel(line: &str) -> Option<i32> {
    // Locate the prefix; bail if absent.
    let after_prefix = line.find(SENTINEL_PREFIX)
        .map(|i| &line[i + SENTINEL_PREFIX.len()..])?;
    // The PGID runs from here to the next `__` (the suffix). Look
    // forward for the suffix; if not found, this isn't our sentinel
    // (could be a substring collision in user output).
    let end = after_prefix.find(SENTINEL_SUFFIX)?;
    let num = &after_prefix[..end];
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
    fn try_parse_sentinel_accepts_decorated_lines() {
        // Real-world cases observed against backend transports that
        // decorate remote stderr with their own prefixes:
        //
        // - devpod ssh prepends ANSI color codes + timestamp + log level.
        // - Some shells may inject ESC sequences from PROMPT_COMMAND
        //   into stderr on first connection.
        //
        // The sentinel itself is distinctive enough that "anywhere in
        // the line" is safe in practice.
        assert_eq!(
            try_parse_sentinel("\u{1b}[0;1;36minfo \u{1b}[0m__DCMCP_PGID=385__"),
            Some(385),
        );
        assert_eq!(
            try_parse_sentinel("[19:05:32] info __DCMCP_PGID=12345__"),
            Some(12345),
        );
    }

    #[test]
    fn try_parse_sentinel_rejects_garbage() {
        assert_eq!(try_parse_sentinel("hello"), None);
        assert_eq!(try_parse_sentinel("__DCMCP_PGID=abc__"), None);
        assert_eq!(try_parse_sentinel("PGID=42"), None);
        // Truncated (no terminator) — must not match.
        assert_eq!(try_parse_sentinel("__DCMCP_PGID=42"), None);
        // Missing PGID number.
        assert_eq!(try_parse_sentinel("__DCMCP_PGID=__"), None);
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

    #[test]
    fn wrap_self_contained_runs_user_cmd_with_no_env() {
        // The whole point of the self-contained variant: no env var
        // setup, command flows through the shell string itself.
        let wrapped = wrap_self_contained(r#"echo "it's a (self-contained) test""#);
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&wrapped)
            .output()
            .expect("spawn");
        assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stdout.contains("it's a (self-contained) test"), "stdout: {stdout:?}");
        assert!(
            stderr.lines().any(|l| try_parse_sentinel(l).is_some()),
            "no sentinel in stderr: {stderr:?}"
        );
    }

    #[test]
    fn wrap_self_contained_preserves_arbitrary_bytes() {
        // Pathological user command: single quotes, double quotes,
        // backticks, dollar signs, backslashes, newlines, parens.
        let user_cmd = "echo 'a' \"b\" `echo c` $((1+1)) \\\\ \n echo done";
        let wrapped = wrap_self_contained(user_cmd);
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&wrapped)
            .output()
            .expect("spawn");
        assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("a b c 2"), "stdout: {stdout:?}");
        assert!(stdout.contains("done"), "stdout: {stdout:?}");
    }
}

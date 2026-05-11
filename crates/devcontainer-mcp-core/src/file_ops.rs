//! Shared logic for file operations inside containers.
//!
//! Provides server-side editing (old_str → new_str replacement), line-number
//! formatting, and helpers to build shell commands for reading/writing files
//! through any backend (DevPod SSH, devcontainer exec, Codespaces SSH).

use std::borrow::Cow;

use base64::{engine::general_purpose::STANDARD, Engine};
use shell_escape::escape;

use crate::error::{Error, Result};

/// Format file content with line numbers, optionally restricting to a range.
///
/// Line numbers are 1-based.  Passing `None` for both bounds returns the
/// entire file.
pub fn format_with_line_numbers(
    content: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let start = start_line.unwrap_or(1).max(1);
    let end = end_line.unwrap_or(total).min(total);

    let mut output = String::new();
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        if n >= start && n <= end {
            output.push_str(&format!("{n}. {line}\n"));
        }
    }
    output
}

/// Apply a surgical edit: find **exactly one** occurrence of `old_str` and
/// replace it with `new_str`.
///
/// Returns an error if `old_str` is not found or appears more than once.
pub fn apply_edit(content: &str, old_str: &str, new_str: &str) -> Result<String> {
    let count = content.matches(old_str).count();
    if count == 0 {
        return Err(Error::FileEdit(
            "old_str not found in file content".to_string(),
        ));
    }
    if count > 1 {
        return Err(Error::FileEdit(format!(
            "old_str found {count} times — must match exactly once. \
             Include more surrounding context to make it unique."
        )));
    }
    Ok(content.replacen(old_str, new_str, 1))
}

/// Shell-escape a string for safe embedding in a shell command.
fn quote(s: &str) -> String {
    escape(Cow::Borrowed(s)).into_owned()
}

/// Build a shell command that reads a file via `cat`.
pub fn read_file_command(path: &str) -> String {
    format!("cat {}", quote(path))
}

/// Build a shell command that writes base64-encoded content to a file,
/// creating parent directories as needed.
pub fn write_file_command(path: &str, content: &str) -> String {
    let path = quote(path);
    let encoded = STANDARD.encode(content.as_bytes());
    format!("mkdir -p \"$(dirname {path})\" && printf '%s' '{encoded}' | base64 -d > {path}")
}

/// Build a shell command that lists a directory (non-hidden, up to 2 levels).
pub fn list_dir_command(path: &str) -> String {
    format!("find {} -maxdepth 2 -not -path '*/.*' | sort", quote(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_line_numbers_full() {
        let content = "line1\nline2\nline3\n";
        let result = format_with_line_numbers(content, None, None);
        assert_eq!(result, "1. line1\n2. line2\n3. line3\n");
    }

    #[test]
    fn test_format_line_numbers_range() {
        let content = "a\nb\nc\nd\ne\n";
        let result = format_with_line_numbers(content, Some(2), Some(4));
        assert_eq!(result, "2. b\n3. c\n4. d\n");
    }

    #[test]
    fn test_apply_edit_success() {
        let content = "fn old_name() {\n    42\n}\n";
        let result = apply_edit(content, "old_name", "new_name").unwrap();
        assert_eq!(result, "fn new_name() {\n    42\n}\n");
    }

    #[test]
    fn test_apply_edit_not_found() {
        let content = "hello world";
        let result = apply_edit(content, "xyz", "abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_apply_edit_multiple_matches() {
        let content = "aaa bbb aaa";
        let result = apply_edit(content, "aaa", "ccc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("2 times"));
    }

    #[test]
    fn test_quote_simple_path() {
        assert_eq!(quote("simple"), "simple");
    }

    #[test]
    fn test_quote_path_with_spaces() {
        let result = quote("path with spaces");
        assert!(result.contains('\'') || result.contains('\\'));
    }

    #[test]
    fn test_quote_path_with_single_quote() {
        let result = quote("it's");
        // Should not break when used in a shell command
        assert!(!result.contains("it's") || result.contains("\\'"));
    }

    #[test]
    fn test_quote_path_with_dollar() {
        let result = quote("$HOME/file");
        // Should be escaped so $HOME is not expanded
        assert_ne!(result, "$HOME/file");
    }
}

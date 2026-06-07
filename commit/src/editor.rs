// SPDX-License-Identifier: MIT
use std::fs::OpenOptions;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// Disambiguates temp files between concurrent `edit` calls in one process.
static EDIT_SEQ: AtomicU64 = AtomicU64::new(0);

/// Resolve the user's preferred editor: `$VISUAL`, then `$EDITOR`, then `vi`.
pub fn resolve_editor() -> String {
    std::env::var("VISUAL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("EDITOR").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "vi".to_string())
}

/// Open the user's editor on `message` and return the edited text (trailing
/// whitespace trimmed). The editor is invoked through `sh -c` exactly like git,
/// so values carrying arguments (e.g. `code --wait`) work.
pub fn edit(message: &str) -> Result<String, String> {
    edit_with(&resolve_editor(), message)
}

/// Core of [`edit`] with an explicit editor command (keeps the global-env
/// resolution out of the file/launch logic so it is testable without env races).
fn edit_with(editor: &str, message: &str) -> Result<String, String> {
    // Unique temp path; PID + per-call sequence avoids collisions between
    // concurrent processes and concurrent calls within one process.
    let seq = EDIT_SEQ.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("aish-COMMIT_EDITMSG-{}-{seq}", std::process::id()));
    std::fs::write(&path, message).map_err(|e| format!("failed to write temp message file: {e}"))?;

    let result = launch(editor, &path);
    let edited = std::fs::read_to_string(&path).map_err(|e| format!("failed to read edited message: {e}"));
    let _ = std::fs::remove_file(&path);
    result?;

    Ok(edited?.trim_end().to_string())
}

/// Run `sh -c '<editor> "$@"' aish <file>` so the path reaches the editor as a
/// single safely-quoted argument regardless of spaces or shell metacharacters.
///
/// The plugin's own stdout is the host protocol pipe, so a full-screen editor
/// must talk to `/dev/tty` directly; otherwise its terminal rendering would
/// corrupt the JSON frame stream. tty redirection is best-effort: when no tty
/// is available (CI, non-interactive) the child inherits the plugin's stdio.
fn launch(editor: &str, path: &Path) -> Result<(), String> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(format!("{editor} \"$@\"")).arg("aish").arg(path);

    if let Ok(tty_in) = OpenOptions::new().read(true).open("/dev/tty") {
        cmd.stdin(tty_in);
    }
    if let Ok(tty_out) = OpenOptions::new().write(true).open("/dev/tty") {
        if let Ok(tty_err) = tty_out.try_clone() {
            cmd.stderr(tty_err);
        }
        cmd.stdout(tty_out);
    }

    let status = cmd
        .status()
        .map_err(|e| format!("failed to launch editor `{editor}`: {e}"))?;
    if !status.success() {
        return Err(format!("editor `{editor}` exited with a non-zero status"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_prefers_visual_over_editor() {
        std::env::set_var("VISUAL", "vis");
        std::env::set_var("EDITOR", "ed");
        assert_eq!(resolve_editor(), "vis");
        std::env::remove_var("VISUAL");
        assert_eq!(resolve_editor(), "ed");
        std::env::remove_var("EDITOR");
        assert_eq!(resolve_editor(), "vi");
    }

    #[test]
    fn edit_returns_editor_modified_content() {
        // A non-interactive "editor" that overwrites the file with new text.
        let out = edit_with("printf 'fix: edited subject' >", "feat: original").unwrap();
        assert_eq!(out, "fix: edited subject");
    }

    #[test]
    fn edit_trims_trailing_whitespace() {
        let out = edit_with("printf 'feat: x\\n\\n' >", "seed").unwrap();
        assert_eq!(out, "feat: x");
    }

    #[test]
    fn edit_surfaces_failure_when_editor_exits_nonzero() {
        let err = edit_with("false", "seed").unwrap_err();
        assert!(err.contains("non-zero status"));
    }
}

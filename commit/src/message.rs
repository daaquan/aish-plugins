// SPDX-License-Identifier: MIT
use crate::protocol::WireMessage;

pub const MAX_DIFF_CHARS: usize = 12_000;

fn floor_char_boundary(s: &str, max: usize) -> usize {
    if max >= s.len() {
        return s.len();
    }
    let mut i = max;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Build the system+user messages for commit-message generation.
pub fn build_messages(style: &str, language: &str, diff: &str) -> Vec<WireMessage> {
    let system = format!(
        "You write git commit messages.\n\
         Style: {style} (when 'conventional', use Conventional Commits: \
         `type(scope): subject`, types feat|fix|refactor|docs|test|chore|perf|ci).\n\
         Language: {language}.\n\
         Output ONLY the commit message. Subject <= 50 chars, imperative mood. \
         No backticks, no explanation, no surrounding quotes."
    );
    let diff = if diff.len() > MAX_DIFF_CHARS {
        let cut = floor_char_boundary(diff, MAX_DIFF_CHARS);
        format!("{}\n[diff truncated]", &diff[..cut])
    } else {
        diff.to_string()
    };
    let user = format!("Generate a commit message for this staged diff:\n\n{diff}");
    vec![
        WireMessage { role: "system".into(), content: system },
        WireMessage { role: "user".into(), content: user },
    ]
}

/// Clean a raw model response: strip surrounding ``` fences and trim.
pub fn postprocess(raw: &str) -> String {
    let mut s = raw.trim();
    if s.starts_with("```") {
        if let Some(idx) = s.find('\n') {
            s = &s[idx + 1..];
        }
        if let Some(idx) = s.rfind("```") {
            s = &s[..idx];
        }
    }
    let result = s.trim();
    if result.contains("```") || result.is_empty() {
        return String::new();
    }
    result.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_style_language_and_diff() {
        let msgs = build_messages("conventional", "en", "diff --git a/x");
        assert_eq!(msgs[0].role, "system");
        assert!(msgs[0].content.to_lowercase().contains("conventional"));
        assert!(msgs[0].content.contains("en"));
        assert!(msgs[1].content.contains("diff --git a/x"));
    }

    #[test]
    fn strips_code_fences_and_trims() {
        assert_eq!(postprocess("```\nfeat: add x\n```\n"), "feat: add x");
    }

    #[test]
    fn strips_language_tagged_fence() {
        assert_eq!(postprocess("```text\nfix: y\n```"), "fix: y");
    }

    #[test]
    fn rejects_empty_output() {
        assert!(postprocess("   \n  ").is_empty());
    }

    #[test]
    fn bare_fence_becomes_empty() {
        assert!(postprocess("```").is_empty());
        assert!(postprocess("``````").is_empty());
        assert!(postprocess("```\n```").is_empty());
    }

    #[test]
    fn caps_huge_diff() {
        let big = "x".repeat(MAX_DIFF_CHARS + 500);
        let msgs = build_messages("conventional", "en", &big);
        assert!(msgs[1].content.contains("[diff truncated]"));
        assert!(msgs[1].content.len() < MAX_DIFF_CHARS + 200);
    }

    #[test]
    fn caps_huge_diff_on_char_boundary_without_panic() {
        let big = "あ".repeat(MAX_DIFF_CHARS);
        let msgs = build_messages("conventional", "en", &big);
        assert!(msgs[1].content.contains("[diff truncated]"));
    }
}

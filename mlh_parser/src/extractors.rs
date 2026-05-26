//! Extracts trailers (Signed-off-by, Reviewed-by, etc.) and patch diffs from
//! email body text.

use regex::Regex;
use std::sync::LazyLock;

use crate::Attribution;
use crate::address_parser::normalize_address;

static RE_COPYPASTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(\S+:\s+[\da-f]+\s+\([^)]+)\n([^\n]+\))")
        .expect("RE_COPYPASTE regex must compile")
});

static RE_WRAPPED_SIGNATURE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(\S+:\s+[^<]+)\n(<[^>]+>)$").expect("RE_WRAPPED_SIGNATURE regex must compile")
});

static RE_SIGNATURE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?P<type>[a-zA-Z\-]+-by):\s*(?P<name>[^<\n]+?)\s*<(?P<email>[^>\n]+)>")
        .expect("RE_SIGNATURE regex must compile")
});

/// Extracts git-style trailer lines from a commit message / email body.
///
/// Matches patterns like `Signed-off-by: Name <email>` and `Reviewed-by: Name <email>`.
/// Handles common copy-paste line wrapping and broken signature lines.
pub fn extract_attributions(commit_message: &str) -> Vec<Attribution> {
    let mut attributions = Vec::new();

    // Split on signature marker
    let body = commit_message.split("\n-- \n").next().unwrap_or("");

    // Fix common copypaste trailer wrapping
    let body = RE_COPYPASTE.replace_all(body, "$1 $2");

    // Fix line broken signature: Signed-off-by: Long Name\n<email.here@example.com>
    let body = RE_WRAPPED_SIGNATURE.replace_all(&body, "$1 $2");

    for caps in RE_SIGNATURE.captures_iter(&body) {
        let attr_type = caps.name("type").map_or("", |m| m.as_str()).trim();
        let name = caps.name("name").map_or("", |m| m.as_str()).trim();
        let email = caps.name("email").map_or("", |m| m.as_str()).trim();
        let identification = normalize_address(&format!("{} <{}>", name, email));
        attributions.push(Attribution {
            attribution: attr_type.to_string(),
            identification,
        });
    }

    attributions
}

static RE_DIFF_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?im)^diff --git ").expect("RE_DIFF_BLOCK must compile"));

/// Extracts patch diffs from an email body.
///
/// Adapted from B4's `LoreMessage.get_body_parts()` and DIFF_RE detection.
/// Splits the body on `---` separators (the git-format-patch commit/diff
/// boundary). Each section that contains `diff --git` content is treated as
/// a separate patch. Patches without a preceding `---` (commit-less diffs)
/// are also handled.
///
/// Multiple patches (multiple `---` sections) in a single body are returned
/// as separate entries. Multiple `diff --git` blocks within a single `---`
/// section are kept together as one patch (multi-file patches).
///
/// Source: https://github.com/mricon/b4/blob/main/src/b4/__init__.py
/// Licensed under GPLv2
pub fn extract_patches(email_body: &str) -> Vec<String> {
    if !RE_DIFF_BLOCK.is_match(email_body) {
        return Vec::new();
    }

    let sep_re = match regex::Regex::new(r"(?m)^---\s*$") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let sep_positions: Vec<usize> = sep_re.find_iter(email_body).map(|m| m.start()).collect();

    let mut starts: Vec<usize> = Vec::new();

    for &pos in &sep_positions {
        starts.push(pos);
    }

    if RE_DIFF_BLOCK.is_match(email_body) {
        let body_before_first_sep = if let Some(&first_sep) = sep_positions.first() {
            &email_body[..first_sep]
        } else {
            email_body
        };
        if sep_positions.is_empty() || RE_DIFF_BLOCK.is_match(body_before_first_sep) {
            starts.push(0);
        }
    }

    starts.sort();
    starts.dedup();

    let mut patches = Vec::new();
    for i in 0..starts.len() {
        let start = starts[i];
        let end = if i + 1 < starts.len() {
            starts[i + 1]
        } else {
            email_body.len()
        };

        let section = &email_body[start..end];

        if RE_DIFF_BLOCK.is_match(section) {
            let patch = section.trim().to_string();
            if !patch.is_empty() {
                patches.push(patch);
            }
        }
    }

    patches
}

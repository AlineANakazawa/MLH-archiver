//! Anonymization functions for applying SHA-1 hashing to identity data.

use regex::Regex;
use sha1::{Digest, Sha1};
use std::borrow::Cow;
use std::sync::LazyLock;

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+(?:\.[a-zA-Z]{2,})?").unwrap());

static BRACKET_IDENTITY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([^<]*?)\s*<([^>]+@[^>]+)>").unwrap());

/// Compute the SHA-1 hex digest of a string.
pub fn generate_sha1_hash(input: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Find the last separator character (`,`, `;`, `:`) in `text_before` and
/// split it into a (name, prefix) pair. The prefix includes the separator and
/// any trailing whitespace, while the name is the text after it (trimmed).
fn extract_name_and_prefix(text_before: &str) -> (String, String) {
    if text_before.is_empty() {
        return (String::new(), String::new());
    }

    let sep_pos = text_before.rmatch_indices([':', ',', ';']).next();

    if let Some((idx, _)) = sep_pos {
        let sep_end = idx + 1;
        let bytes = text_before.as_bytes();
        let mut ws_end = sep_end;
        while ws_end < bytes.len() && (bytes[ws_end] == b' ' || bytes[ws_end] == b'\t') {
            ws_end += 1;
        }
        let prefix = &text_before[..ws_end];
        let name = text_before[ws_end..].trim();
        (name.to_string(), prefix.to_string())
    } else {
        let name = text_before.trim();
        (name.to_string(), String::new())
    }
}

/// Callback for BRACKET_IDENTITY_RE replacement.
/// Hashes the name (if present) and the email inside angle brackets.
fn replace_bracketed_identity(caps: &regex::Captures) -> String {
    let before = caps.get(1).map_or("", |m| m.as_str());
    let email = caps.get(2).map_or("", |m| m.as_str());

    let (name, prefix) = extract_name_and_prefix(before);

    if !name.is_empty() {
        format!(
            "{}{} <{}>",
            prefix,
            generate_sha1_hash(&name),
            generate_sha1_hash(email)
        )
    } else {
        format!("{}<{}>", before, generate_sha1_hash(email))
    }
}

/// Process a single line: replace bracketed identities first, then bare emails.
fn anonymize_line(line: &str, is_multiline: bool) -> String {
    let line = BRACKET_IDENTITY_RE.replace_all(line, replace_bracketed_identity);

    let matches: Vec<_> = EMAIL_RE.find_iter(&line).collect();
    if matches.is_empty() {
        return line.into_owned();
    }

    // Collect line bytes for boundary checks
    let line_bytes = line.as_bytes();
    let line_len = line_bytes.len();

    let mut result = String::with_capacity(line.len());
    let mut last_end = 0;

    for m in &matches {
        let start = m.start();
        let end = m.end();
        let email = m.as_str();

        if start > 0 && line_bytes[start - 1] == b'<' {
            continue;
        }
        if end < line_len && line_bytes[end] == b'>' {
            continue;
        }

        let text_before = &line[last_end..start];

        if is_multiline && text_before.trim().is_empty() {
            result.push_str(text_before);
            result.push_str(email);
            last_end = end;
            continue;
        }

        let (name, prefix) = extract_name_and_prefix(text_before);

        if !name.is_empty() {
            result.push_str(&prefix);
            result.push_str(&generate_sha1_hash(&name));
            result.push_str(" <");
            result.push_str(&generate_sha1_hash(email));
            result.push('>');
        } else {
            result.push_str(text_before);
            result.push_str(&generate_sha1_hash(email));
        }

        last_end = end;
    }

    result.push_str(&line[last_end..]);
    result
}

/// Apply SHA-1 anonymization to identities within a string.
///
/// Detects `Name <email>` patterns and bare email addresses, replacing them
/// with SHA-1 hashes. Processes line by line to avoid matching identities
/// split across multiple lines.
///
/// Returns the string unchanged if no identities are found.
pub fn anonymize_string(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.split('\n').collect();
    let is_multiline = lines.len() > 1;

    let result_lines: Vec<String> = lines
        .into_iter()
        .map(|line| anonymize_line(line, is_multiline))
        .collect();

    result_lines.join("\n")
}

/// Like `anonymize_string` but returns `Cow::Borrowed` when no identities are found,
/// avoiding allocation for text that doesn't contain emails or names.
pub fn maybe_anonymize(v: &str) -> Cow<'_, str> {
    let hashed = anonymize_string(v);
    if hashed.as_str() == v {
        Cow::Borrowed(v)
    } else {
        Cow::Owned(hashed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_known_value() {
        assert_eq!(
            generate_sha1_hash("test"),
            "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(anonymize_string(""), "");
    }

    #[test]
    fn test_no_identity() {
        assert_eq!(anonymize_string("hello world"), "hello world");
    }
}

//! Search utilities: FTS5 query building and BM25 score normalization.

/// Sanitize a term for FTS5 (keep only alphanumeric + apostrophes).
fn sanitize_fts5_term(term: &str) -> String {
    term.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'')
        .collect::<String>()
        .to_lowercase()
}

/// Build an FTS5 query from user-facing search syntax.
///
/// Supports quoted phrases, negation (`-term`), and prefix matching.
/// Returns `None` if no usable terms.
///
/// # Examples
///
/// ```
/// use qmd::search::build_fts5_query;
///
/// assert_eq!(
///     build_fts5_query("performance -sports"),
///     Some(r#""performance"* NOT "sports"*"#.to_string()),
/// );
/// ```
#[must_use]
pub fn build_fts5_query(query: &str) -> Option<String> {
    let mut positive: Vec<String> = Vec::new();
    let mut negative: Vec<String> = Vec::new();

    let s = query.trim();
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let negated = bytes[i] == b'-';
        if negated {
            i += 1;
            if i >= bytes.len() {
                break;
            }
        }

        if bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            let phrase = &s[start..i];
            if i < bytes.len() {
                i += 1;
            }
            let sanitized: String = phrase
                .split_whitespace()
                .map(sanitize_fts5_term)
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if !sanitized.is_empty() {
                let fts = format!("\"{sanitized}\"");
                if negated {
                    &mut negative
                } else {
                    &mut positive
                }
                .push(fts);
            }
        } else {
            let start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'"' {
                i += 1;
            }
            let sanitized = sanitize_fts5_term(&s[start..i]);
            if !sanitized.is_empty() {
                let fts = format!("\"{sanitized}\"*");
                if negated {
                    &mut negative
                } else {
                    &mut positive
                }
                .push(fts);
            }
        }
    }

    if positive.is_empty() {
        return None;
    }

    let mut result = positive.join(" ");
    for neg in &negative {
        result = format!("{result} NOT {neg}");
    }
    Some(result)
}

/// Monotonic mapping of raw BM25 to `[0, 1)`: `x / (1 + x)`.
#[must_use]
pub fn normalize_bm25(score: f64) -> f64 {
    let s = score.abs();
    s / (1.0 + s)
}

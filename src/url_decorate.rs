/// Convert an https://... URL into a filesystem-friendly filename.
///
/// - Keeps it readable (host + path + query-ish parts)
/// - Avoids path separators and other illegal/reserved characters
/// - Trims trailing dots/spaces (problematic on Windows)
/// - Falls back to a short hash if the result would be empty
pub fn url_to_filename(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let s = url.strip_prefix("https://").unwrap_or(url);

    let mut out = String::with_capacity(s.len());

    // Windows reserved device names. We'll avoid producing these exactly.
    // (Case-insensitive; with optional extension they’re still problematic in practice.)
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL",
        "COM1","COM2","COM3","COM4","COM5","COM6","COM7","COM8","COM9",
        "LPT1","LPT2","LPT3","LPT4","LPT5","LPT6","LPT7","LPT8","LPT9",
    ];

    // Replace percent-escapes with a safe marker rather than trying to decode.
    // (Decoding can introduce '/' or other problematic chars again.)
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // Commonly problematic on Windows/macOS/Linux
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),

            // Control chars => underscore
            c if c.is_control() => out.push('_'),

            // Keep a conservative allowed set:
            // alnum, dot, dash, underscore are generally safe everywhere.
            c if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' => out.push(c),

            // Some URL-ish separators we often want readable
            '&' | '=' | '+' | '@' => out.push('-'),

            // Percent sign: keep but normalize runs like "%2F"
            '%' => out.push_str("_pct_"),

            // Everything else (including unicode) => underscore
            _ => out.push('_'),
        }

        // Optional: collapse multiple underscores to a single underscore
        if out.ends_with("__") {
            while out.ends_with("__") {
                out.pop();
            }
        }
    }

    // Trim problematic trailing characters (Windows forbids trailing space/dot)
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }

    // Ensure non-empty
    if out.is_empty() {
        let mut h = DefaultHasher::new();
        url.hash(&mut h);
        return format!("url_{:016x}", h.finish());
    }

    // Avoid reserved device names (case-insensitive) by prefixing underscore
    let upper = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED.iter().any(|&r| r == upper) {
        out.insert(0, '_');
    }

    format!("{out}__{:08x}.html", crc32fast::hash(url.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::url_to_filename;

    #[test]
    fn basic() {
        let f = url_to_filename("https://example.com/a/b?x=1&y=2");
        assert!(f.contains("example.com"));
        assert!(!f.contains('/'));
        assert!(!f.contains('?'));
        assert!(!f.contains('&'));
        assert!(!f.contains('='));
    }

    #[test]
    fn reserved() {
        assert_eq!(url_to_filename("https://con"), "_con");
        assert_eq!(url_to_filename("https://NUL.txt"), "_NUL.txt"); // will be "_NUL.txt" then you can lowercase externally if desired
    }
}

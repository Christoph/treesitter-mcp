use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum CountLanguage {
    CLike,
    JavaScript,
    Rust,
    Python,
    Plain,
}

pub fn language_for_path(path: &Path) -> CountLanguage {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return CountLanguage::Plain;
    };

    match ext.to_ascii_lowercase().as_str() {
        "rs" => CountLanguage::Rust,
        "py" => CountLanguage::Python,
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => CountLanguage::JavaScript,
        "go" | "java" | "cs" | "c" | "h" | "cpp" | "hpp" | "cc" => CountLanguage::CLike,
        _ => CountLanguage::Plain,
    }
}

/// Count all identifier-like words in a file and accumulate into the provided map.
/// This is useful for single-pass extraction+counting.
pub fn count_words_in_content(
    content: &str,
    language: CountLanguage,
    word_counts: &mut HashMap<String, usize>,
) {
    let stripped = strip_comments_and_strings(content, language);
    for word in stripped.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if !word.is_empty() {
            *word_counts.entry(word.to_string()).or_insert(0) += 1;
        }
    }
}

pub fn strip_comments_and_strings(content: &str, language: CountLanguage) -> String {
    match language {
        CountLanguage::Plain => content.to_string(),
        CountLanguage::Python => strip_with_config(
            content,
            StripConfig {
                line_comment: Some('#'),
                c_like_comments: false,
                allow_single_quote: true,
                allow_double_quote: true,
                allow_triple_quote: true,
                allow_backtick: false,
                allow_rust_raw_strings: false,
            },
        ),
        CountLanguage::JavaScript => strip_with_config(
            content,
            StripConfig {
                line_comment: Some('/'),
                c_like_comments: true,
                allow_single_quote: true,
                allow_double_quote: true,
                allow_triple_quote: false,
                allow_backtick: true,
                allow_rust_raw_strings: false,
            },
        ),
        CountLanguage::Rust => strip_with_config(
            content,
            StripConfig {
                line_comment: Some('/'),
                c_like_comments: true,
                allow_single_quote: true,
                allow_double_quote: true,
                allow_triple_quote: false,
                allow_backtick: false,
                allow_rust_raw_strings: true,
            },
        ),
        CountLanguage::CLike => strip_with_config(
            content,
            StripConfig {
                line_comment: Some('/'),
                c_like_comments: true,
                allow_single_quote: true,
                allow_double_quote: true,
                allow_triple_quote: false,
                allow_backtick: false,
                allow_rust_raw_strings: false,
            },
        ),
    }
}

#[derive(Debug, Clone, Copy)]
struct StripConfig {
    line_comment: Option<char>,
    c_like_comments: bool,
    allow_single_quote: bool,
    allow_double_quote: bool,
    allow_triple_quote: bool,
    allow_backtick: bool,
    allow_rust_raw_strings: bool,
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    Code,
    LineComment,
    BlockComment,
    String { quote: char, triple: bool },
    Backtick,
    RustRaw { hashes: usize },
}

fn strip_with_config(content: &str, config: StripConfig) -> String {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    let mut mode = Mode::Code;

    while i < bytes.len() {
        let b = bytes[i];
        let ch = b as char;

        match mode {
            Mode::Code => {
                if config.allow_rust_raw_strings && b == b'r' {
                    if let Some((hashes, consumed)) = detect_rust_raw_string_start(bytes, i) {
                        // Replace prefix with spaces
                        for _ in 0..consumed {
                            out.push(' ');
                        }
                        i += consumed;
                        mode = Mode::RustRaw { hashes };
                        continue;
                    }
                }

                if config.c_like_comments && b == b'/' && i + 1 < bytes.len() {
                    if bytes[i + 1] == b'/' {
                        out.push(' ');
                        out.push(' ');
                        i += 2;
                        mode = Mode::LineComment;
                        continue;
                    }
                    if bytes[i + 1] == b'*' {
                        out.push(' ');
                        out.push(' ');
                        i += 2;
                        mode = Mode::BlockComment;
                        continue;
                    }
                }

                if config.line_comment == Some('#') && b == b'#' {
                    out.push(' ');
                    i += 1;
                    mode = Mode::LineComment;
                    continue;
                }

                if config.allow_backtick && b == b'`' {
                    out.push(' ');
                    i += 1;
                    mode = Mode::Backtick;
                    continue;
                }

                if config.allow_double_quote && b == b'"' {
                    if config.allow_triple_quote && starts_with(bytes, i, b"\"\"\"") {
                        out.push_str("   ");
                        i += 3;
                        mode = Mode::String {
                            quote: '"',
                            triple: true,
                        };
                        continue;
                    }
                    out.push(' ');
                    i += 1;
                    mode = Mode::String {
                        quote: '"',
                        triple: false,
                    };
                    continue;
                }

                if config.allow_single_quote && b == b'\'' {
                    if config.allow_triple_quote && starts_with(bytes, i, b"'''") {
                        out.push_str("   ");
                        i += 3;
                        mode = Mode::String {
                            quote: '\'',
                            triple: true,
                        };
                        continue;
                    }
                    out.push(' ');
                    i += 1;
                    mode = Mode::String {
                        quote: '\'',
                        triple: false,
                    };
                    continue;
                }

                out.push(ch);
                i += 1;
            }
            Mode::LineComment => {
                if b == b'\n' {
                    out.push('\n');
                    i += 1;
                    mode = Mode::Code;
                } else {
                    out.push(' ');
                    i += 1;
                }
            }
            Mode::BlockComment => {
                if b == b'\n' {
                    out.push('\n');
                    i += 1;
                    continue;
                }

                if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    mode = Mode::Code;
                    continue;
                }

                out.push(' ');
                i += 1;
            }
            Mode::Backtick => {
                if b == b'\n' {
                    out.push('\n');
                    i += 1;
                    continue;
                }

                if b == b'\\' {
                    out.push(' ');
                    if i + 1 < bytes.len() {
                        out.push(' ');
                        i += 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }

                if b == b'`' {
                    out.push(' ');
                    i += 1;
                    mode = Mode::Code;
                    continue;
                }

                out.push(' ');
                i += 1;
            }
            Mode::String { quote, triple } => {
                if b == b'\n' {
                    out.push('\n');
                    i += 1;
                    continue;
                }

                if b == b'\\' {
                    out.push(' ');
                    if i + 1 < bytes.len() {
                        out.push(' ');
                        i += 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }

                if triple {
                    if quote == '"' && starts_with(bytes, i, b"\"\"\"") {
                        out.push_str("   ");
                        i += 3;
                        mode = Mode::Code;
                        continue;
                    }
                    if quote == '\'' && starts_with(bytes, i, b"'''") {
                        out.push_str("   ");
                        i += 3;
                        mode = Mode::Code;
                        continue;
                    }
                    out.push(' ');
                    i += 1;
                    continue;
                }

                if ch == quote {
                    out.push(' ');
                    i += 1;
                    mode = Mode::Code;
                    continue;
                }

                out.push(' ');
                i += 1;
            }
            Mode::RustRaw { hashes } => {
                if b == b'\n' {
                    out.push('\n');
                    i += 1;
                    continue;
                }

                if b == b'"' && ends_rust_raw_string(bytes, i, hashes) {
                    // Replace closing delimiter
                    out.push(' ');
                    for _ in 0..hashes {
                        out.push(' ');
                    }
                    i += 1 + hashes;
                    mode = Mode::Code;
                    continue;
                }

                out.push(' ');
                i += 1;
            }
        }
    }

    out
}

fn starts_with(haystack: &[u8], at: usize, needle: &[u8]) -> bool {
    haystack.get(at..at + needle.len()) == Some(needle)
}

fn detect_rust_raw_string_start(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    // r"..." or r#"..."# etc.
    if bytes.get(start) != Some(&b'r') {
        return None;
    }

    let mut i = start + 1;
    let mut hashes = 0;
    while i < bytes.len() && bytes[i] == b'#' {
        hashes += 1;
        i += 1;
    }

    if i < bytes.len() && bytes[i] == b'"' {
        let consumed = 2 + hashes; // r + hashes + "
        return Some((hashes, consumed));
    }

    None
}

fn ends_rust_raw_string(bytes: &[u8], quote_index: usize, hashes: usize) -> bool {
    if bytes.get(quote_index) != Some(&b'"') {
        return false;
    }

    for j in 0..hashes {
        if bytes.get(quote_index + 1 + j) != Some(&b'#') {
            return false;
        }
    }

    true
}

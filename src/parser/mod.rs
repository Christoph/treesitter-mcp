use eyre::{bail, Result};
use std::path::Path;
use tree_sitter::{Parser, Tree};

/// Supported programming languages for tree-sitter parsing
///
/// Each language has a corresponding tree-sitter grammar that can parse
/// source code into a concrete syntax tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// Rust programming language (.rs)
    Rust,
    /// Python programming language (.py)
    Python,
    /// JavaScript (.js, .mjs, .cjs)
    JavaScript,
    /// TypeScript (.ts, .tsx)
    TypeScript,
    /// HTML markup (.html, .htm)
    Html,
    /// CSS stylesheets (.css)
    Css,
    /// Swift programming language (.swift)
    Swift,
    /// C# programming language (.cs)
    CSharp,
    /// Java programming language (.java)
    Java,
    /// Go programming language (.go)
    Go,
    /// Kotlin programming language (.kt, .kts)
    Kotlin,
}

impl Language {
    /// Get a human-readable name for the language
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Html => "HTML",
            Language::Css => "CSS",
            Language::Swift => "Swift",
            Language::CSharp => "C#",
            Language::Java => "Java",
            Language::Go => "Go",
            Language::Kotlin => "Kotlin",
        }
    }

    /// Get the tree-sitter language grammar for this language
    pub fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Html => tree_sitter_html::LANGUAGE.into(),
            Language::Css => tree_sitter_css::LANGUAGE.into(),
            Language::Swift => tree_sitter_swift::LANGUAGE.into(),
            Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Language::Java => tree_sitter_java::LANGUAGE.into(),
            Language::Go => tree_sitter_go::LANGUAGE.into(),
            Language::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
        }
    }
}

/// Detect programming language from file path extension
///
/// This function examines the file extension and maps it to a supported
/// language. The detection is case-insensitive.
///
/// # Supported Extensions
/// - `.rs` → Rust
/// - `.py` → Python
/// - `.js`, `.mjs`, `.cjs` → JavaScript
/// - `.ts`, `.tsx` → TypeScript
/// - `.html`, `.htm` → HTML
/// - `.css` → CSS
/// - `.swift` → Swift
/// - `.cs` → C#
/// - `.java` → Java
/// - `.go` → Go
/// - `.kt`, `.kts` → Kotlin
///
/// # Arguments
/// * `path` - File path (can be absolute, relative, or just a filename)
///
/// # Errors
/// Returns an error if:
/// - The file has no extension
/// - The extension is not supported
///
/// # Examples
/// ```
/// use treesitter_mcp::parser::{detect_language, Language};
///
/// let lang = detect_language("src/main.rs").unwrap();
/// assert_eq!(lang, Language::Rust);
///
/// let lang = detect_language("script.py").unwrap();
/// assert_eq!(lang, Language::Python);
///
/// // Case insensitive
/// let lang = detect_language("Test.RS").unwrap();
/// assert_eq!(lang, Language::Rust);
///
/// // Unsupported extension
/// assert!(detect_language("file.txt").is_err());
/// ```
pub fn detect_language(path: impl AsRef<Path>) -> Result<Language> {
    let path = path.as_ref();

    // Extract and normalize extension to lowercase
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        Some("rs") => Ok(Language::Rust),
        Some("py") => Ok(Language::Python),
        Some("js") | Some("mjs") | Some("cjs") => Ok(Language::JavaScript),
        Some("ts") | Some("tsx") => Ok(Language::TypeScript),
        Some("html") | Some("htm") => Ok(Language::Html),
        Some("css") => Ok(Language::Css),
        Some("swift") => Ok(Language::Swift),
        Some("cs") => Ok(Language::CSharp),
        Some("java") => Ok(Language::Java),
        Some("go") => Ok(Language::Go),
        Some("kt") | Some("kts") => Ok(Language::Kotlin),
        Some(ext) => {
            bail!("Unsupported file extension: .{}", ext)
        }
        None => {
            bail!("No file extension found in path: {}", path.display())
        }
    }
}

/// Parse source code into a tree-sitter syntax tree
///
/// Creates a concrete syntax tree (CST) from the source code using the
/// appropriate tree-sitter grammar for the language.
///
/// # Arguments
/// * `source` - Source code to parse
/// * `language` - Programming language of the source code
///
/// # Returns
/// Returns a `Tree` representing the parsed syntax tree. Even if the source
/// contains syntax errors, a tree is still returned with error nodes marked.
///
/// # Errors
/// Returns an error if:
/// - The parser cannot be configured for the language
/// - The parser fails completely (very rare)
///
/// # Examples
/// ```
/// use treesitter_mcp::parser::{parse_code, Language};
///
/// let code = "fn main() { println!(\"hello\"); }";
/// let tree = parse_code(code, Language::Rust).unwrap();
///
/// let root = tree.root_node();
/// assert_eq!(root.kind(), "source_file");
/// assert!(!root.has_error());
/// ```
pub fn parse_code(source: &str, language: Language) -> Result<Tree> {
    log::debug!("Parsing {} code ({} bytes)", language.name(), source.len());

    // Create a new parser instance
    let mut parser = Parser::new();

    // Configure parser for the specific language
    parser.set_language(&language.tree_sitter_language())?;

    // Parse the source code
    // Note: Even invalid syntax produces a tree with error nodes
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| eyre::eyre!("Failed to parse {} code", language.name()))?;

    if tree.root_node().has_error() {
        log::warn!("Parse tree contains syntax errors");
    }

    Ok(tree)
}

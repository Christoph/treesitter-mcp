# Plan: HTML, CSS (Tailwind v4), and Askama Template Support

## Overview

Add tree-sitter analysis support for HTML and CSS to help LLMs create consistent UIs by exposing:
- Available custom component classes (e.g., `btn-primary`, `card`, `input`)
- Theme variables (e.g., `--color-primary`, `--spacing-lg`)
- Class usage patterns in HTML
- Askama template structure with optional merging

## Decisions

| Decision | Choice |
|----------|--------|
| HTML extraction focus | IDs and custom classes only |
| CSS extraction focus | `@theme` variables + custom `@layer` classes only |
| Tailwind version | v4 only (`@import "tailwindcss"`, `@theme`, `@layer`) |
| Embedded script/style | Raw text (no deep parsing) |
| Class output format | Name + full selector context |
| Template dir detection | Auto-detect (`templates/` in parent dirs, max depth 10) |
| Merged template output | Option A: merged content + dependency list with types |
| @apply/@theme parsing | Regex (tree-sitter won't handle these non-standard directives) |
| JSON output format | Minified (compact for token efficiency) |

---

## Part 1: Data Structures

### CSS Shape (`src/analysis/shape.rs`)

```rust
use std::borrow::Cow;

/// Theme variable from @theme block
#[derive(Debug, serde::Serialize, Clone)]
pub struct ThemeVariable {
    pub name: String,   // "--color-primary", "--spacing-lg"
    pub value: String,  // "oklch(0.6 0.2 250)", "1.5rem"
    pub line: usize,
}

/// Custom component class (defined with @apply or custom styles)
#[derive(Debug, serde::Serialize, Clone)]
pub struct CustomClass {
    pub name: String,                    // "btn-primary", "card"
    pub applied_utilities: Vec<String>,  // ["bg-primary", "text-white", "px-4"]
    pub layer: Option<Cow<'static, str>>, // "components", "utilities", or None
    pub line: usize,
}

/// Keyframe animation
#[derive(Debug, serde::Serialize, Clone)]
pub struct KeyframeInfo {
    pub name: String,
    pub line: usize,
}

/// CSS file shape (Tailwind v4 focused)
#[derive(Debug, serde::Serialize)]
pub struct CssFileShape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// Theme variables from @theme block
    pub theme: Vec<ThemeVariable>,
    
    /// Custom component/utility classes (reusable)
    pub custom_classes: Vec<CustomClass>,
    
    /// @keyframes animations
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<KeyframeInfo>,
}
```

### HTML Shape (`src/analysis/shape.rs`)

```rust
/// HTML element with id
#[derive(Debug, serde::Serialize, Clone)]
pub struct HtmlIdInfo {
    pub tag: String,
    pub id: String,
    pub line: usize,
}

/// Script reference
#[derive(Debug, serde::Serialize, Clone)]
pub struct ScriptInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    pub inline: bool,
    pub line: usize,
}

/// Style reference
#[derive(Debug, serde::Serialize, Clone)]
pub struct StyleInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    pub inline: bool,
    pub line: usize,
}

/// HTML file shape
#[derive(Debug, serde::Serialize)]
pub struct HtmlFileShape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// Elements with IDs (for JS/navigation)
    pub ids: Vec<HtmlIdInfo>,
    
    /// All unique custom classes used (non-Tailwind utilities)
    pub classes_used: Vec<String>,
    
    /// Script references
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<ScriptInfo>,
    
    /// Style references
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<StyleInfo>,
}
```

### Askama Template Shape (`src/analysis/file_shape.rs`)

```rust
/// Template dependency info
#[derive(Debug, serde::Serialize, Clone)]
pub struct TemplateDependency {
    pub path: String,
    pub dependency_type: String,  // "extends" or "include"
}

/// Template file shape (when merge_templates=true)
#[derive(Debug, serde::Serialize)]
pub struct MergedTemplateShape {
    pub path: String,
    pub merged_content: String,
    pub dependencies: Vec<TemplateDependency>,
}
```

---

## Part 2: Tailwind Utility Detection

```rust
/// Check if a class name is a Tailwind utility (to filter out)
/// 
/// NOTE: This list covers common Tailwind v4 utilities but is not exhaustive.
/// It may need updates as Tailwind evolves. Consider making this configurable
/// in the future to allow users to add custom utility patterns.
fn is_tailwind_utility(class: &str) -> bool {
    // Handle important modifier at the start
    let class = class.strip_prefix('!').unwrap_or(class);
    
    // Handle variant prefixes (hover:, dark:, sm:, etc.)
    let base = class.split(':').last().unwrap_or(class);
    
    // Exact match utilities
    let exact = [
        // Layout
        "flex", "grid", "block", "inline", "inline-block", "inline-flex", "inline-grid",
        "hidden", "container", "table", "table-row", "table-cell",
        // Position
        "relative", "absolute", "fixed", "sticky", "static",
        // Display
        "visible", "invisible", "collapse",
        // Accessibility
        "sr-only", "not-sr-only",
        // Interactivity
        "pointer-events-none", "pointer-events-auto",
        // Other common utilities
        "truncate", "italic", "underline", "line-through", "no-underline",
        "uppercase", "lowercase", "capitalize", "normal-case",
    ];
    if exact.contains(&base) {
        return true;
    }
    
    // Prefix-based utilities
    let prefixes = [
        // Spacing
        "p-", "px-", "py-", "pt-", "pr-", "pb-", "pl-", "ps-", "pe-",
        "m-", "mx-", "my-", "mt-", "mr-", "mb-", "ml-", "ms-", "me-", "-m",
        "gap-", "space-",
        // Sizing
        "w-", "h-", "min-w-", "min-h-", "max-w-", "max-h-", "size-",
        // Typography
        "text-", "font-", "leading-", "tracking-", "indent-",
        "decoration-", "underline-offset-",
        // Colors
        "bg-", "from-", "via-", "to-", "fill-", "stroke-",
        "text-", "border-", "outline-", "ring-", "shadow-",
        // Borders
        "border-", "rounded-", "shadow-", "ring-", "outline-", "divide-",
        // Layout
        "flex-", "grid-", "col-", "row-", "order-",
        "items-", "justify-", "content-", "place-", "self-",
        "auto-cols-", "auto-rows-",
        // Position
        "z-", "top-", "right-", "bottom-", "left-", "inset-",
        // Transforms
        "scale-", "rotate-", "translate-", "skew-", "origin-",
        // Transitions & Animations
        "transition-", "duration-", "delay-", "ease-", "animate-",
        // Effects
        "opacity-", "mix-blend-", "bg-blend-",
        "backdrop-blur-", "backdrop-brightness-", "backdrop-contrast-",
        "backdrop-grayscale-", "backdrop-hue-rotate-", "backdrop-invert-",
        "backdrop-opacity-", "backdrop-saturate-", "backdrop-sepia-",
        // Filters
        "blur-", "brightness-", "contrast-", "drop-shadow-",
        "grayscale-", "hue-rotate-", "invert-", "saturate-", "sepia-",
        // Interactivity
        "cursor-", "pointer-events-", "resize-", "select-", "user-select-",
        "caret-", "accent-",
        // Overflow
        "overflow-", "overscroll-", "scroll-", "snap-",
        // Other
        "aspect-", "columns-", "break-", "break-after-", "break-before-", "break-inside-",
        "float-", "clear-", "object-", "isolation-",
        "list-", "placeholder-", "will-change-", "touch-",
    ];
    
    prefixes.iter().any(|p| base.starts_with(p))
        || base.contains('[')  // Arbitrary values like w-[300px]
}
```

---

## Part 3: CSS Extraction (Regex-based for Tailwind)

```rust
use regex::Regex;
use std::io;

/// Extract CSS shape from Tailwind v4 source code
/// 
/// This function uses regex to parse Tailwind-specific directives (@theme, @layer, @apply)
/// which are not part of standard CSS and thus not handled by tree-sitter-css.
fn extract_css_tailwind(source: &str, file_path: Option<&str>) -> Result<CssFileShape, io::Error> {
    let mut theme = Vec::new();
    let mut custom_classes = Vec::new();
    let mut keyframes = Vec::new();
    
    // 1. Extract @theme block variables
    let theme_block_re = Regex::new(r"@theme\s*\{([\s\S]*?)\}")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    if let Some(cap) = theme_block_re.captures(source) {
        let theme_start = cap.get(0).unwrap().start();
        let theme_content = &cap[1];
        
        let var_re = Regex::new(r"(?m)^\s*(--[\w-]+)\s*:\s*([^;]+);")
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
        
        for var_cap in var_re.captures_iter(theme_content) {
            let var_start_in_theme = var_cap.get(0).unwrap().start();
            let absolute_offset = theme_start + var_start_in_theme;
            
            theme.push(ThemeVariable {
                name: var_cap[1].to_string(),
                value: var_cap[2].trim().to_string(),
                line: calculate_line(source, absolute_offset),
            });
        }
    }
    
    // 2. Extract @layer components/utilities blocks
    let layer_re = Regex::new(r"@layer\s+(components|utilities)\s*\{([\s\S]*?)\}")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    for layer_cap in layer_re.captures_iter(source) {
        let layer_name = match &layer_cap[1] {
            "components" => Cow::Borrowed("components"),
            "utilities" => Cow::Borrowed("utilities"),
            _ => Cow::Owned(layer_cap[1].to_string()),
        };
        let layer_start = layer_cap.get(0).unwrap().start();
        let layer_content = &layer_cap[2];
        
        // Extract class definitions within layer
        // Note: This simple regex won't handle nested braces (e.g., media queries)
        // For production use, consider a more robust parser
        let class_re = Regex::new(r"\.([\w-]+)\s*\{([^}]*)\}")
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
        
        for class_cap in class_re.captures_iter(layer_content) {
            let class_start_in_layer = class_cap.get(0).unwrap().start();
            let absolute_offset = layer_start + class_start_in_layer;
            let class_name = class_cap[1].to_string();
            let class_body = &class_cap[2];
            
            // Extract @apply utilities
            let mut applied = Vec::new();
            let apply_re = Regex::new(r"@apply\s+([^;]+);")
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
            
            for apply_cap in apply_re.captures_iter(class_body) {
                applied.extend(
                    apply_cap[1].split_whitespace().map(String::from)
                );
            }
            
            custom_classes.push(CustomClass {
                name: class_name,
                applied_utilities: applied,
                layer: Some(layer_name.clone()),
                line: calculate_line(source, absolute_offset),
            });
        }
    }
    
    // 3. Extract @keyframes
    let keyframes_re = Regex::new(r"@keyframes\s+([\w-]+)\s*\{")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    for kf_cap in keyframes_re.captures_iter(source) {
        keyframes.push(KeyframeInfo {
            name: kf_cap[1].to_string(),
            line: calculate_line(source, kf_cap.get(0).unwrap().start()),
        });
    }
    
    Ok(CssFileShape {
        path: file_path.map(String::from),
        theme,
        custom_classes,
        keyframes,
    })
}

/// Calculate line number from byte offset
fn calculate_line(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].matches('\n').count() + 1
}
```

---

## Part 4: HTML Extraction (Tree-sitter)

```rust
use tree_sitter::{Query, QueryCursor, Tree};
use std::collections::HashSet;

/// Extract HTML shape from parsed tree
fn extract_html_shape(tree: &Tree, source: &str, file_path: Option<&str>) -> Result<HtmlFileShape, io::Error> {
    let mut ids = Vec::new();
    let mut all_classes = Vec::new();
    let mut scripts = Vec::new();
    let mut styles = Vec::new();
    
    // Tree-sitter queries for HTML
    let query = Query::new(
        &tree_sitter_html::LANGUAGE.into(),
        r#"
        ; Capture all elements with attributes
        (element
          (start_tag
            (tag_name) @tag
            (attribute
              (attribute_name) @attr_name
              (quoted_attribute_value (attribute_value) @attr_value)))) @element
        
        ; Script elements
        (script_element
          (start_tag) @script_start) @script
        
        ; Style elements  
        (style_element
          (start_tag) @style_start) @style
        
        ; Link elements (for external stylesheets)
        (element
          (start_tag
            (tag_name) @link_tag
            (attribute
              (attribute_name) @link_attr_name
              (quoted_attribute_value (attribute_value) @link_attr_value))))
        "#,
    ).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {e}")))?;
    
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    
    for match_ in matches {
        let mut current_tag = None;
        let mut current_line = 0;
        let mut attrs = std::collections::HashMap::new();
        
        for capture in match_.captures {
            let node = capture.node;
            let capture_name = query.capture_names()[capture.index as usize];
            
            match capture_name {
                "tag" | "link_tag" => {
                    if let Ok(tag_text) = node.utf8_text(source.as_bytes()) {
                        current_tag = Some(tag_text.to_string());
                        current_line = node.start_position().row + 1;
                    }
                }
                "attr_name" | "link_attr_name" => {
                    if let Ok(attr_name) = node.utf8_text(source.as_bytes()) {
                        // Store attribute name for next value
                        if let Some(next_sibling) = node.next_sibling() {
                            if let Ok(attr_value) = next_sibling.utf8_text(source.as_bytes()) {
                                // Remove quotes from attribute value
                                let value = attr_value.trim_matches('"').trim_matches('\'');
                                attrs.insert(attr_name.to_string(), value.to_string());
                            }
                        }
                    }
                }
                "script_start" => {
                    let line = node.start_position().row + 1;
                    // Check for src attribute
                    let src = extract_attribute(&node, source, "src");
                    scripts.push(ScriptInfo {
                        src,
                        inline: src.is_none(),
                        line,
                    });
                }
                "style_start" => {
                    let line = node.start_position().row + 1;
                    styles.push(StyleInfo {
                        href: None,
                        inline: true,
                        line,
                    });
                }
                _ => {}
            }
        }
        
        // Process collected attributes for the current element
        if let Some(tag) = current_tag {
            // Handle id attribute
            if let Some(id_value) = attrs.get("id") {
                ids.push(HtmlIdInfo {
                    tag: tag.clone(),
                    id: id_value.clone(),
                    line: current_line,
                });
            }
            
            // Handle class attribute
            if let Some(class_value) = attrs.get("class") {
                all_classes.extend(
                    class_value.split_whitespace().map(String::from)
                );
            }
            
            // Handle link elements (stylesheets)
            if tag == "link" {
                if let Some(rel) = attrs.get("rel") {
                    if rel == "stylesheet" {
                        if let Some(href) = attrs.get("href") {
                            styles.push(StyleInfo {
                                href: Some(href.clone()),
                                inline: false,
                                line: current_line,
                            });
                        }
                    }
                }
            }
        }
    }
    
    // Deduplicate and filter classes
    let classes_used: Vec<String> = all_classes
        .into_iter()
        .filter(|c| !is_tailwind_utility(c))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    
    Ok(HtmlFileShape {
        path: file_path.map(String::from),
        ids,
        classes_used,
        scripts,
        styles,
    })
}

/// Helper to extract attribute value from a node
fn extract_attribute(node: &tree_sitter::Node, source: &str, attr_name: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute" {
            let mut attr_cursor = child.walk();
            let mut found_name = false;
            for attr_child in child.children(&mut attr_cursor) {
                if attr_child.kind() == "attribute_name" {
                    if let Ok(name) = attr_child.utf8_text(source.as_bytes()) {
                        if name == attr_name {
                            found_name = true;
                        }
                    }
                } else if found_name && attr_child.kind() == "quoted_attribute_value" {
                    if let Ok(value) = attr_child.utf8_text(source.as_bytes()) {
                        return Some(value.trim_matches('"').trim_matches('\'').to_string());
                    }
                }
            }
        }
    }
    None
}
```

---

## Part 5: Askama Template Resolution

### Auto-detect Templates Directory

```rust
use std::path::{Path, PathBuf};

/// Find templates directory by walking up from file path
/// 
/// Searches up to MAX_DEPTH parent directories to avoid performance issues
/// in deeply nested projects.
fn find_templates_dir(file_path: &Path) -> Option<PathBuf> {
    let mut current = file_path.parent()?;
    let mut depth = 0;
    const MAX_DEPTH: usize = 10;
    
    while depth < MAX_DEPTH {
        // Check if current dir is named "templates"
        if current.file_name().map(|n| n == "templates").unwrap_or(false) {
            return Some(current.to_path_buf());
        }
        
        // Check if "templates" subdir exists
        let templates_subdir = current.join("templates");
        if templates_subdir.is_dir() {
            return Some(templates_subdir);
        }
        
        current = current.parent()?;
        depth += 1;
    }
    
    None
}
```

### Template Dependency Detection

```rust
use std::fs;

/// Find template dependencies (extends and includes)
/// 
/// Validates paths to prevent directory traversal attacks.
fn find_template_dependencies(
    source: &str,
    templates_dir: &Path,
) -> Result<Vec<TemplateDependency>, io::Error> {
    let mut deps = Vec::new();
    let canonical_templates = templates_dir.canonicalize()?;
    
    // {% extends "path" %} - supports both single and double quotes
    let extends_re = Regex::new(r#"\{%\s*extends\s+["']([^"']+)["']\s*%\}"#)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    for cap in extends_re.captures_iter(source) {
        let template_path = &cap[1];
        let path = templates_dir.join(template_path);
        
        // Security: Validate path is within templates_dir
        if let Ok(canonical_path) = path.canonicalize() {
            if canonical_path.starts_with(&canonical_templates) && path.exists() {
                deps.push(TemplateDependency {
                    path: template_path.to_string(),
                    dependency_type: "extends".to_string(),
                });
            }
        }
    }
    
    // {% include "path" %} - supports both single and double quotes
    let include_re = Regex::new(r#"\{%\s*include\s+["']([^"']+)["']\s*%\}"#)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    for cap in include_re.captures_iter(source) {
        let template_path = &cap[1];
        let path = templates_dir.join(template_path);
        
        // Security: Validate path is within templates_dir
        if let Ok(canonical_path) = path.canonicalize() {
            if canonical_path.starts_with(&canonical_templates) && path.exists() {
                deps.push(TemplateDependency {
                    path: template_path.to_string(),
                    dependency_type: "include".to_string(),
                });
            }
        }
    }
    
    Ok(deps)
}
```

### Template Merging

```rust
use std::collections::{HashMap, HashSet};

/// Merge template with its dependencies (extends and includes)
/// 
/// Uses separate tracking for visited files and recursion stack to properly
/// handle circular dependencies while allowing the same file to be included
/// multiple times in different branches.
fn merge_template(
    template_path: &Path,
    templates_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    recursion_stack: &mut Vec<PathBuf>,
) -> Result<String, io::Error> {
    let canonical = template_path.canonicalize().map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Template not found: {}", template_path.display())
        )
    })?;
    
    // Check for circular dependency
    if recursion_stack.contains(&canonical) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Circular template dependency detected: {:?}", recursion_stack)
        ));
    }
    
    recursion_stack.push(canonical.clone());
    visited.insert(canonical.clone());
    
    let source = fs::read_to_string(template_path)?;
    
    // Handle {% extends "base.html" %}
    let extends_re = Regex::new(r#"\{%\s*extends\s+["']([^"']+)["']\s*%\}"#)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    if let Some(cap) = extends_re.captures(&source) {
        let parent_path = templates_dir.join(&cap[1]);
        
        // Security: Validate path
        let canonical_parent = parent_path.canonicalize().map_err(|e| {
            io::Error::new(io::ErrorKind::NotFound, format!("Parent template not found: {e}"))
        })?;
        let canonical_templates = templates_dir.canonicalize()?;
        
        if !canonical_parent.starts_with(&canonical_templates) {
            recursion_stack.pop();
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Path traversal detected in template extends"
            ));
        }
        
        let parent_content = merge_template(&parent_path, templates_dir, visited, recursion_stack)?;
        
        // Extract blocks from child
        let child_blocks = extract_blocks(&source)?;
        
        // Replace blocks in parent
        let result = replace_blocks(&parent_content, &child_blocks)?;
        recursion_stack.pop();
        return Ok(result);
    }
    
    // Handle {% include "partial.html" %}
    let include_re = Regex::new(r#"\{%\s*include\s+["']([^"']+)["']\s*%\}"#)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    let result = include_re.replace_all(&source, |caps: &regex::Captures| {
        let include_path = templates_dir.join(&caps[1]);
        
        // Security: Validate path
        if let Ok(canonical_include) = include_path.canonicalize() {
            if let Ok(canonical_templates) = templates_dir.canonicalize() {
                if canonical_include.starts_with(&canonical_templates) {
                    return merge_template(&include_path, templates_dir, visited, recursion_stack)
                        .unwrap_or_else(|e| format!("<!-- include error: {} - {} -->", &caps[1], e));
                }
            }
        }
        format!("<!-- include error: {} - path validation failed -->", &caps[1])
    });
    
    recursion_stack.pop();
    Ok(result.to_string())
}

/// Extract block definitions from template source
fn extract_blocks(source: &str) -> Result<HashMap<String, String>, io::Error> {
    let mut blocks = HashMap::new();
    let block_re = Regex::new(r#"\{%\s*block\s+(\w+)\s*%\}([\s\S]*?)\{%\s*endblock\s*%\}"#)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    for cap in block_re.captures_iter(source) {
        blocks.insert(cap[1].to_string(), cap[2].to_string());
    }
    Ok(blocks)
}

/// Replace block placeholders in parent template with child blocks
fn replace_blocks(parent: &str, child_blocks: &HashMap<String, String>) -> Result<String, io::Error> {
    let block_re = Regex::new(r#"\{%\s*block\s+(\w+)\s*%\}([\s\S]*?)\{%\s*endblock\s*%\}"#)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    
    let result = block_re.replace_all(parent, |caps: &regex::Captures| {
        let block_name = &caps[1];
        // Use child block if exists, otherwise keep parent default
        child_blocks.get(block_name)
            .cloned()
            .unwrap_or_else(|| caps[2].to_string())
    });
    
    Ok(result.to_string())
}
```

---

## Part 6: Integration with Existing Code

### Update `src/analysis/shape.rs`

Add new extraction functions that integrate with existing `extract_enhanced_shape`:

```rust
/// Extract shape based on language type
pub fn extract_enhanced_shape(
    tree: &Tree,
    source: &str,
    language: Language,
    file_path: Option<&str>,
) -> Result<EnhancedFileShape, io::Error> {
    let shape = match language {
        Language::Rust => extract_rust_enhanced(tree, source)?,
        Language::Python => extract_python_enhanced(tree, source)?,
        Language::JavaScript => extract_js_enhanced(tree, source, Language::JavaScript)?,
        Language::TypeScript => extract_js_enhanced(tree, source, Language::TypeScript)?,
        Language::Html => {
            // Return HTML shape wrapped in a compatible format
            let html_shape = extract_html_shape(tree, source, file_path)?;
            return Ok(convert_html_to_enhanced(html_shape));
        }
        Language::Css => {
            // Return CSS shape wrapped in a compatible format
            let css_shape = extract_css_tailwind(source, file_path)?;
            return Ok(convert_css_to_enhanced(css_shape));
        }
        _ => EnhancedFileShape {
            path: None,
            language: None,
            functions: vec![],
            structs: vec![],
            classes: vec![],
            imports: vec![],
        },
    };

    Ok(EnhancedFileShape {
        path: file_path.map(|p| p.to_string()),
        language: Some(language.name().to_string()),
        ..shape
    })
}
```

### Update `src/analysis/file_shape.rs`

Add HTML/CSS handling to `extract_shape`:

```rust
pub fn extract_shape(
    tree: &Tree,
    source: &str,
    language: Language,
) -> Result<FileShape, io::Error> {
    match language {
        Language::Rust => extract_rust_shape(tree, source),
        Language::Python => extract_python_shape(tree, source),
        Language::JavaScript => extract_js_shape(tree, source),
        Language::TypeScript => extract_ts_shape(tree, source),
        Language::Html => {
            let html_shape = crate::analysis::shape::extract_html_shape(tree, source, None)?;
            Ok(convert_html_shape_to_file_shape(html_shape))
        }
        Language::Css => {
            let css_shape = crate::analysis::shape::extract_css_tailwind(source, None)?;
            Ok(convert_css_shape_to_file_shape(css_shape))
        }
        _ => Ok(FileShape {
            path: None,
            functions: vec![],
            structs: vec![],
            classes: vec![],
            imports: vec![],
            dependencies: vec![],
        }),
    }
}
```

### Update `src/tools.rs`

Add `merge_templates` parameter with proper error handling:

```rust
Tool {
    name: "file_shape".to_string(),
    description: "Extract file structure without implementation details".to_string(),
    input_schema: json!({
        "type": "object",
        "properties": {
            "file_path": {
                "type": "string",
                "description": "Path to the source file"
            },
            "include_deps": {
                "type": "boolean",
                "description": "Include project dependencies as nested shapes",
                "default": false
            },
            "merge_templates": {
                "type": "boolean",
                "description": "For Askama/Jinja2 templates (.html in templates/ dir): merge extends/includes into single output. Returns error if used on non-template files.",
                "default": false
            }
        },
        "required": ["file_path"]
    }),
}
```

Update execution logic in `file_shape.rs`:

```rust
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path_str = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let include_deps = arguments["include_deps"].as_bool().unwrap_or(false);
    let merge_templates = arguments["merge_templates"].as_bool().unwrap_or(false);

    log::info!("Extracting shape of file: {file_path_str} (include_deps: {include_deps}, merge_templates: {merge_templates})");

    let path = Path::new(file_path_str);
    
    // Handle template merging if requested
    if merge_templates {
        // Validate this is a template file
        let templates_dir = find_templates_dir(path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "merge_templates=true requires file to be in a 'templates/' directory"
            )
        })?;
        
        let source = fs::read_to_string(path)?;
        let mut visited = HashSet::new();
        let mut recursion_stack = Vec::new();
        
        let merged_content = merge_template(path, &templates_dir, &mut visited, &mut recursion_stack)?;
        let dependencies = find_template_dependencies(&source, &templates_dir)?;
        
        let merged_shape = MergedTemplateShape {
            path: path.to_string_lossy().to_string(),
            merged_content,
            dependencies,
        };
        
        let shape_json = serde_json::to_string(&merged_shape).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize merged template: {e}"),
            )
        })?;
        
        return Ok(CallToolResult::success(shape_json));
    }

    // ... rest of existing logic ...
}
```

---

## Part 7: Test Fixtures

### Minimal Test Fixtures for Unit Tests

Create minimal fixtures in `tests/fixtures/minimal/`:

**`tests/fixtures/minimal/simple.css`**:
```css
@theme {
  --color-primary: blue;
}

@layer components {
  .btn {
    @apply px-4 py-2;
  }
}
```

**`tests/fixtures/minimal/simple.html`**:
```html
<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
  <div id="main" class="card btn-primary">Content</div>
</body>
</html>
```

### Comprehensive Test Fixtures for Integration Tests

(Keep the existing comprehensive fixtures from Part 7 of the original plan for integration tests)

---

## Part 8: Expected Output Examples

### CSS Shape Output (Minified)

```json
{"path":"styles/globals.css","theme":[{"name":"--color-primary","value":"oklch(0.6 0.2 250)","line":5}],"custom_classes":[{"name":"btn","applied_utilities":["inline-flex","items-center","justify-center","px-4","py-2","rounded-md","font-medium","transition-colors"],"layer":"components","line":38}],"keyframes":[{"name":"fade-in","line":28}]}
```

### HTML Shape Output (Minified)

```json
{"path":"index.html","ids":[{"tag":"header","id":"app-header","line":10}],"classes_used":["btn-primary","card","card-header"],"scripts":[{"src":"./index.js","inline":false,"line":6}],"styles":[{"href":"./styles/globals.css","inline":false,"line":5}]}
```

---

## Part 9: Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add `regex = "1.10"` dependency |
| `src/parser/mod.rs` | Modify | Add `Language::Html` and `Language::Css` variants |
| `src/analysis/shape.rs` | Modify | Add CSS/HTML shapes, extraction functions, integration with `extract_enhanced_shape` |
| `src/analysis/file_shape.rs` | Modify | Add HTML/CSS shape extraction, `merge_templates` param, Askama resolution, error handling |
| `src/analysis/get_context.rs` | Modify | Add HTML/CSS context node types |
| `src/tools.rs` | Modify | Add `merge_templates` parameter to file_shape schema |
| `tests/fixtures/minimal/simple.css` | Create | Minimal CSS test fixture |
| `tests/fixtures/minimal/simple.html` | Create | Minimal HTML test fixture |
| `tests/fixtures/typescript_project/index.html` | Create | Comprehensive HTML test fixture |
| `tests/fixtures/typescript_project/styles/globals.css` | Create | Comprehensive Tailwind v4 CSS fixture |
| `tests/fixtures/rust_project/templates/base.html` | Create | Askama base template |
| `tests/fixtures/rust_project/templates/calculator.html` | Create | Askama child template |
| `tests/fixtures/rust_project/templates/partials/header.html` | Create | Askama partial |
| `tests/fixtures/rust_project/templates/partials/footer.html` | Create | Askama partial |
| `tests/fixtures/rust_project/templates/partials/history.html` | Create | Askama partial |
| `tests/fixtures/rust_project/Cargo.toml` | Modify | Add askama dependency |
| `tests/fixtures/rust_project/src/lib.rs` | Modify | Add `pub mod templates;` |
| `tests/fixtures/rust_project/src/templates.rs` | Create | Template structs |
| `tests/css_extraction_test.rs` | Create | Unit tests for CSS extraction |
| `tests/html_extraction_test.rs` | Create | Unit tests for HTML extraction |
| `tests/askama_merging_test.rs` | Create | Unit tests for Askama template merging |
| `tests/html_css_integration_test.rs` | Create | Integration tests for HTML + CSS together |

---

## Part 10: Implementation Order

| Phase | Tasks | Priority |
|-------|-------|----------|
| 1 | Add `regex = "1.10"` to Cargo.toml dependencies | High |
| 2 | Add `Language::Html` and `Language::Css` to parser | High |
| 3 | Add data structures to `shape.rs` | High |
| 4 | Implement `extract_css_tailwind()` with proper error handling | High |
| 5 | Create minimal CSS test fixtures and unit tests | High |
| 6 | Implement `extract_html_shape()` with tree-sitter + filtering | High |
| 7 | Create minimal HTML test fixtures and unit tests | High |
| 8 | Integrate CSS/HTML extraction with `extract_enhanced_shape()` | High |
| 9 | Update `file_shape.rs` to handle HTML/CSS languages | High |
| 10 | Implement template directory detection with depth limit | Medium |
| 11 | Implement template dependency detection with security validation | High |
| 12 | Implement template merging with circular dependency detection | High |
| 13 | Create Askama template fixtures and unit tests | High |
| 14 | Add `merge_templates` param to `tools.rs` with error handling | Medium |
| 15 | Update `get_context.rs` for HTML/CSS nodes | Medium |
| 16 | Create comprehensive test fixtures for integration tests | Medium |
| 17 | Add integration tests for HTML + CSS together | High |
| 18 | Add edge case tests (minified CSS, circular templates, etc.) | High |
| 19 | Add security tests (path traversal, large files) | High |
| 20 | Add performance tests (large files, deep inheritance) | Medium |
| 21 | Update documentation with usage examples | Medium |

---

## Part 11: Testing Strategy

### Unit Tests

1. **CSS Extraction** (`tests/css_extraction_test.rs`):
   - Test @theme variable extraction
   - Test @layer components/utilities extraction
   - Test @apply directive parsing
   - Test @keyframes extraction
   - Test minified CSS (no whitespace)
   - Test nested braces (should document limitations)
   - Test error handling for invalid regex

2. **HTML Extraction** (`tests/html_extraction_test.rs`):
   - Test ID extraction
   - Test class extraction and filtering
   - Test script/style reference extraction
   - Test malformed HTML handling
   - Test empty/missing attributes

3. **Askama Merging** (`tests/askama_merging_test.rs`):
   - Test simple extends
   - Test nested includes
   - Test block replacement
   - Test circular dependency detection
   - Test path traversal prevention
   - Test missing template handling

### Integration Tests

1. **HTML + CSS** (`tests/html_css_integration_test.rs`):
   - Test extracting both HTML and CSS from a project
   - Test matching custom classes between HTML and CSS
   - Test complete UI component analysis

### Security Tests

1. **Path Traversal**:
   - Test `../../etc/passwd` in template includes
   - Test absolute paths in template references

2. **DoS Prevention**:
   - Test extremely large CSS files (>10MB)
   - Test deeply nested template inheritance (>20 levels)

### Performance Tests

1. **Large Files**:
   - Test HTML with 10,000+ elements
   - Test CSS with 1,000+ classes

---

## Part 12: Usage Examples

### Extract CSS Shape

```bash
# Using MCP protocol
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"file_shape","arguments":{"file_path":"styles/globals.css"}}}' | treesitter-mcp
```

### Extract HTML Shape

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"file_shape","arguments":{"file_path":"index.html"}}}' | treesitter-mcp
```

### Merge Askama Templates

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"file_shape","arguments":{"file_path":"templates/calculator.html","merge_templates":true}}}' | treesitter-mcp
```

---

## Summary

This implementation helps LLMs create consistent UIs by:

1. **Exposing design tokens** from `@theme` block (colors, spacing, fonts, etc.)
2. **Exposing reusable component classes** with their Tailwind utility composition
3. **Tracking custom class usage** in HTML to understand patterns
4. **Providing merged template views** for complete UI structure understanding

The focus is specifically on what helps with UI consistency - not raw Tailwind utilities (which are just noise), but the semantic layer built on top of Tailwind.

### Key Improvements from Review

1. **Security**: Added path traversal validation for template includes
2. **Error Handling**: Proper error propagation with `Result<T, io::Error>` throughout
3. **Regex Safety**: All regex compilations use `map_err` instead of `unwrap()`
4. **Byte Offset Calculation**: Fixed to account for parent match offsets
5. **Circular Dependency Detection**: Separate recursion stack from visited set
6. **Tailwind Utility Detection**: Expanded list with documentation about maintenance
7. **Performance**: Added depth limit to template directory search
8. **Integration**: Clear integration points with existing codebase
9. **Testing**: Comprehensive test strategy including security and performance tests
10. **Documentation**: Added usage examples and clear error messages

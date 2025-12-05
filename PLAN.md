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
| Template dir detection | Auto-detect (`templates/` in parent dirs) |
| Merged template output | Option A: merged content + dependency list with types |
| @apply/@theme parsing | Regex (tree-sitter won't handle these non-standard directives) |

---

## Part 1: Data Structures

### CSS Shape (`src/analysis/shape.rs`)

```rust
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
    pub layer: Option<String>,           // "components", "utilities", or None
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
fn is_tailwind_utility(class: &str) -> bool {
    // Handle variant prefixes (hover:, dark:, sm:, etc.)
    let base = class.split(':').last().unwrap_or(class);
    
    // Exact match utilities
    let exact = [
        "flex", "grid", "block", "inline", "hidden", "container",
        "relative", "absolute", "fixed", "sticky", "static",
        "visible", "invisible", "collapse",
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
        "w-", "h-", "min-", "max-", "size-",
        // Typography
        "text-", "font-", "leading-", "tracking-", "indent-",
        // Colors
        "bg-", "from-", "via-", "to-", "fill-", "stroke-",
        // Borders
        "border", "rounded", "shadow", "ring", "outline", "divide-",
        // Layout
        "flex-", "grid-", "col-", "row-", "order-",
        "items-", "justify-", "content-", "place-", "self-",
        // Position
        "z-", "top-", "right-", "bottom-", "left-", "inset-",
        // Transforms
        "scale-", "rotate-", "translate-", "skew-", "origin-",
        // Transitions
        "transition", "duration-", "delay-", "ease-",
        // Other
        "opacity-", "animate-", "cursor-", "pointer-events-", "select-",
        "overflow-", "overscroll-", "scroll-", "snap-",
        "aspect-", "columns-", "break-", "float-", "clear-", "object-",
    ];
    
    prefixes.iter().any(|p| base.starts_with(p))
        || base.contains('[')  // Arbitrary values like w-[300px]
        || base.starts_with("!")  // Important modifier
}
```

---

## Part 3: CSS Extraction (Regex-based for Tailwind)

```rust
use regex::Regex;

fn extract_css_tailwind(source: &str, file_path: Option<&str>) -> CssFileShape {
    let mut theme = Vec::new();
    let mut custom_classes = Vec::new();
    let mut keyframes = Vec::new();
    
    // 1. Extract @theme block variables
    let theme_block_re = Regex::new(r"@theme\s*\{([\s\S]*?)\n\}").unwrap();
    if let Some(cap) = theme_block_re.captures(source) {
        let theme_content = &cap[1];
        let var_re = Regex::new(r"(?m)^\s*(--[\w-]+)\s*:\s*([^;]+);").unwrap();
        for (line_offset, var_cap) in var_re.captures_iter(theme_content).enumerate() {
            theme.push(ThemeVariable {
                name: var_cap[1].to_string(),
                value: var_cap[2].trim().to_string(),
                line: calculate_line(source, var_cap.get(0).unwrap().start()),
            });
        }
    }
    
    // 2. Extract @layer components/utilities blocks
    let layer_re = Regex::new(r"@layer\s+(components|utilities)\s*\{([\s\S]*?)\n\}").unwrap();
    for layer_cap in layer_re.captures_iter(source) {
        let layer_name = layer_cap[1].to_string();
        let layer_content = &layer_cap[2];
        
        // Extract class definitions within layer
        let class_re = Regex::new(r"\.([\w-]+)\s*\{([^}]*)\}").unwrap();
        for class_cap in class_re.captures_iter(layer_content) {
            let class_name = class_cap[1].to_string();
            let class_body = &class_cap[2];
            
            // Extract @apply utilities
            let mut applied = Vec::new();
            let apply_re = Regex::new(r"@apply\s+([^;]+);").unwrap();
            for apply_cap in apply_re.captures_iter(class_body) {
                applied.extend(
                    apply_cap[1].split_whitespace().map(String::from)
                );
            }
            
            custom_classes.push(CustomClass {
                name: class_name,
                applied_utilities: applied,
                layer: Some(layer_name.clone()),
                line: calculate_line(source, class_cap.get(0).unwrap().start()),
            });
        }
    }
    
    // 3. Extract @keyframes (use tree-sitter or regex)
    let keyframes_re = Regex::new(r"@keyframes\s+([\w-]+)\s*\{").unwrap();
    for kf_cap in keyframes_re.captures_iter(source) {
        keyframes.push(KeyframeInfo {
            name: kf_cap[1].to_string(),
            line: calculate_line(source, kf_cap.get(0).unwrap().start()),
        });
    }
    
    CssFileShape {
        path: file_path.map(String::from),
        theme,
        custom_classes,
        keyframes,
    }
}

fn calculate_line(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].matches('\n').count() + 1
}
```

---

## Part 4: HTML Extraction (Tree-sitter)

```rust
fn extract_html_shape(tree: &Tree, source: &str, file_path: Option<&str>) -> HtmlFileShape {
    let mut ids = Vec::new();
    let mut all_classes = Vec::new();
    let mut scripts = Vec::new();
    let mut styles = Vec::new();
    
    // Tree-sitter queries for HTML
    let query = Query::new(
        &tree_sitter_html::LANGUAGE.into(),
        r#"
        ; Elements with id attribute
        (element
          (start_tag
            (tag_name) @tag
            (attribute
              (attribute_name) @attr_name
              (quoted_attribute_value (attribute_value) @attr_value))))
        
        ; Script elements
        (script_element
          (start_tag) @script_start) @script
        
        ; Style elements  
        (style_element) @style
        
        ; Link elements
        (element
          (start_tag
            (tag_name) @link_tag
            (#eq? @link_tag "link"))) @link
        "#,
    ).unwrap();
    
    // Process matches to extract:
    // - id attributes -> ids
    // - class attributes -> filter through is_tailwind_utility() -> all_classes
    // - script src or inline -> scripts
    // - style href or inline -> styles
    
    // Deduplicate and filter classes
    let classes_used: Vec<String> = all_classes
        .into_iter()
        .filter(|c| !is_tailwind_utility(c))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    
    HtmlFileShape {
        path: file_path.map(String::from),
        ids,
        classes_used,
        scripts,
        styles,
    }
}
```

---

## Part 5: Askama Template Resolution

### Auto-detect Templates Directory

```rust
fn find_templates_dir(file_path: &Path) -> Option<PathBuf> {
    let mut current = file_path.parent()?;
    
    loop {
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
    }
}
```

### Template Dependency Detection

```rust
fn find_template_dependencies(source: &str, templates_dir: &Path) -> Vec<TemplateDependency> {
    let mut deps = Vec::new();
    
    // {% extends "path" %}
    let extends_re = Regex::new(r#"\{%\s*extends\s+"([^"]+)"\s*%\}"#).unwrap();
    for cap in extends_re.captures_iter(source) {
        let path = templates_dir.join(&cap[1]);
        if path.exists() {
            deps.push(TemplateDependency {
                path: cap[1].to_string(),
                dependency_type: "extends".to_string(),
            });
        }
    }
    
    // {% include "path" %}
    let include_re = Regex::new(r#"\{%\s*include\s+"([^"]+)"\s*%\}"#).unwrap();
    for cap in include_re.captures_iter(source) {
        let path = templates_dir.join(&cap[1]);
        if path.exists() {
            deps.push(TemplateDependency {
                path: cap[1].to_string(),
                dependency_type: "include".to_string(),
            });
        }
    }
    
    deps
}
```

### Template Merging

```rust
fn merge_template(
    template_path: &Path,
    templates_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<String, io::Error> {
    let canonical = fs::canonicalize(template_path)?;
    if visited.contains(&canonical) {
        return Ok(String::new()); // Prevent infinite recursion
    }
    visited.insert(canonical);
    
    let source = fs::read_to_string(template_path)?;
    
    // Handle {% extends "base.html" %}
    let extends_re = Regex::new(r#"\{%\s*extends\s+"([^"]+)"\s*%\}"#).unwrap();
    if let Some(cap) = extends_re.captures(&source) {
        let parent_path = templates_dir.join(&cap[1]);
        let parent_content = merge_template(&parent_path, templates_dir, visited)?;
        
        // Extract blocks from child
        let child_blocks = extract_blocks(&source);
        
        // Replace blocks in parent
        return Ok(replace_blocks(&parent_content, &child_blocks));
    }
    
    // Handle {% include "partial.html" %}
    let include_re = Regex::new(r#"\{%\s*include\s+"([^"]+)"\s*%\}"#).unwrap();
    let result = include_re.replace_all(&source, |caps: &regex::Captures| {
        let include_path = templates_dir.join(&caps[1]);
        merge_template(&include_path, templates_dir, visited)
            .unwrap_or_else(|_| format!("<!-- include error: {} -->", &caps[1]))
    });
    
    Ok(result.to_string())
}

fn extract_blocks(source: &str) -> HashMap<String, String> {
    let mut blocks = HashMap::new();
    let block_re = Regex::new(
        r#"\{%\s*block\s+(\w+)\s*%\}([\s\S]*?)\{%\s*endblock\s*%\}"#
    ).unwrap();
    
    for cap in block_re.captures_iter(source) {
        blocks.insert(cap[1].to_string(), cap[2].to_string());
    }
    blocks
}

fn replace_blocks(parent: &str, child_blocks: &HashMap<String, String>) -> String {
    let block_re = Regex::new(
        r#"\{%\s*block\s+(\w+)\s*%\}([\s\S]*?)\{%\s*endblock\s*%\}"#
    ).unwrap();
    
    block_re.replace_all(parent, |caps: &regex::Captures| {
        let block_name = &caps[1];
        // Use child block if exists, otherwise keep parent default
        child_blocks.get(block_name)
            .cloned()
            .unwrap_or_else(|| caps[2].to_string())
    }).to_string()
}
```

---

## Part 6: Tool Parameter Updates

### file_shape Tool (`src/tools.rs`)

Add `merge_templates` parameter:

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
                "description": "For Askama/Jinja2 templates: merge extends/includes into single output",
                "default": false
            }
        },
        "required": ["file_path"]
    }),
}
```

---

## Part 7: Test Fixtures

### `tests/fixtures/typescript_project/styles/globals.css`

```css
@import "tailwindcss";

@theme {
  /* Colors */
  --color-primary: oklch(0.6 0.2 250);
  --color-primary-hover: oklch(0.5 0.25 250);
  --color-secondary: oklch(0.7 0.15 160);
  --color-success: oklch(0.7 0.2 145);
  --color-warning: oklch(0.8 0.15 85);
  --color-error: oklch(0.65 0.25 25);
  
  /* Spacing */
  --spacing-xs: 0.25rem;
  --spacing-sm: 0.5rem;
  --spacing-md: 1rem;
  --spacing-lg: 1.5rem;
  --spacing-xl: 2rem;
  
  /* Typography */
  --font-display: "Inter", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", monospace;
  
  /* Radius */
  --radius-sm: 0.25rem;
  --radius-md: 0.5rem;
  --radius-lg: 1rem;
  
  /* Animations */
  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  
  @keyframes slide-up {
    from { transform: translateY(10px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
}

@layer components {
  .btn {
    @apply inline-flex items-center justify-center px-4 py-2 rounded-md font-medium transition-colors;
  }
  
  .btn-primary {
    @apply btn bg-primary text-white hover:bg-primary-hover;
  }
  
  .btn-secondary {
    @apply btn bg-secondary text-white;
  }
  
  .btn-outline {
    @apply btn border-2 border-primary text-primary bg-transparent hover:bg-primary hover:text-white;
  }
  
  .card {
    @apply bg-white rounded-lg shadow-md p-6;
  }
  
  .card-header {
    @apply border-b pb-4 mb-4;
  }
  
  .card-title {
    @apply text-xl font-semibold;
  }
  
  .input {
    @apply w-full px-4 py-2 rounded-md border border-gray-300 focus:ring-2 focus:ring-primary;
  }
  
  .label {
    @apply block text-sm font-medium text-gray-700 mb-1;
  }
  
  .calc-display {
    @apply w-full p-4 text-right text-3xl font-mono bg-gray-900 text-green-400 rounded-t-lg;
  }
  
  .calc-key {
    @apply p-4 text-xl font-bold rounded transition-colors hover:bg-gray-300;
  }
  
  .calc-key-operator {
    @apply calc-key bg-primary text-white hover:bg-primary-hover;
  }
}

@layer utilities {
  .text-balance {
    text-wrap: balance;
  }
  
  .animate-fade-in {
    animation: fade-in 0.3s ease-out;
  }
  
  .animate-slide-up {
    animation: slide-up 0.4s ease-out;
  }
}
```

### `tests/fixtures/typescript_project/index.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Calculator App</title>
    <link rel="stylesheet" href="./styles/globals.css">
    <script src="./index.js" type="module" defer></script>
</head>
<body class="min-h-screen bg-gray-50">
    <header id="app-header" class="bg-white border-b">
        <div class="container mx-auto px-4 py-4 flex items-center justify-between">
            <h1 class="text-2xl font-bold text-primary">Calculator</h1>
            <nav id="main-nav" class="flex gap-6">
                <a href="#basic">Basic</a>
                <a href="#advanced">Advanced</a>
            </nav>
        </div>
    </header>
    
    <main id="app-content" class="container mx-auto px-4 py-8">
        <section id="basic" class="card max-w-md mx-auto mb-8">
            <div class="card-header">
                <h2 class="card-title">Basic Calculator</h2>
            </div>
            <div id="calc-display" class="calc-display">0</div>
            <div id="calc-keypad" class="grid grid-cols-4 gap-1 p-2">
                <button class="calc-key">7</button>
                <button class="calc-key">8</button>
                <button class="calc-key">9</button>
                <button class="calc-key-operator">÷</button>
                <button class="calc-key">4</button>
                <button class="calc-key">5</button>
                <button class="calc-key">6</button>
                <button class="calc-key-operator">×</button>
                <button class="calc-key">1</button>
                <button class="calc-key">2</button>
                <button class="calc-key">3</button>
                <button class="calc-key-operator">−</button>
                <button class="calc-key">0</button>
                <button class="calc-key">.</button>
                <button class="calc-key-operator">=</button>
                <button class="calc-key-operator">+</button>
            </div>
            <button id="clear-btn" class="btn-secondary mt-4">Clear</button>
        </section>
        
        <section id="advanced" class="card max-w-md mx-auto">
            <div class="card-header">
                <h2 class="card-title">Advanced Operations</h2>
            </div>
            <form id="custom-operation" class="space-y-4">
                <div>
                    <label for="input-a" class="label">First Number</label>
                    <input type="number" id="input-a" class="input">
                </div>
                <div>
                    <label for="input-b" class="label">Second Number</label>
                    <input type="number" id="input-b" class="input">
                </div>
                <button type="submit" class="btn-primary w-full">Calculate</button>
            </form>
        </section>
    </main>
    
    <footer id="app-footer" class="border-t mt-auto py-6 text-center">
        <p class="text-sm text-gray-600">Built with TypeScript</p>
    </footer>
</body>
</html>
```

### `tests/fixtures/rust_project/templates/base.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>{% block title %}Calculator{% endblock %}</title>
    <link rel="stylesheet" href="/static/globals.css">
    {% block head %}{% endblock %}
</head>
<body class="min-h-screen bg-gray-50">
    {% include "partials/header.html" %}
    
    <main id="app-content" class="container mx-auto px-4 py-8">
        {% block content %}{% endblock %}
    </main>
    
    {% include "partials/footer.html" %}
    
    {% block scripts %}{% endblock %}
</body>
</html>
```

### `tests/fixtures/rust_project/templates/calculator.html`

```html
{% extends "base.html" %}

{% block title %}{{ operation }} - Calculator{% endblock %}

{% block content %}
<section id="calculator" class="card max-w-md mx-auto">
    <div class="card-header">
        <h2 class="card-title">{{ operation }}</h2>
    </div>
    
    <div id="result-display" class="p-6 bg-gray-100 rounded-lg text-center mb-4">
        {% match result %}
            {% when Some with (value) %}
                <p class="text-sm text-gray-500 mb-1">Result</p>
                <span class="text-4xl font-mono font-bold text-primary">{{ value }}</span>
            {% when None %}
                <span class="text-2xl text-error">Error</span>
        {% endmatch %}
    </div>
    
    <form action="/calculate" method="post" class="space-y-4">
        <div class="grid grid-cols-3 gap-4">
            <div>
                <label for="input-a" class="label">A</label>
                <input type="number" name="a" id="input-a" value="{{ a }}" class="input text-center">
            </div>
            <div>
                <select name="op" class="input text-center text-xl">
                    {% for op in operations %}
                    <option value="{{ op.value }}"{% if op.value == selected_op %} selected{% endif %}>
                        {{ op.label }}
                    </option>
                    {% endfor %}
                </select>
            </div>
            <div>
                <label for="input-b" class="label">B</label>
                <input type="number" name="b" id="input-b" value="{{ b }}" class="input text-center">
            </div>
        </div>
        <button type="submit" class="btn-primary w-full">Calculate</button>
    </form>
    
    {% if !history.is_empty() %}
    {% include "partials/history.html" %}
    {% endif %}
</section>
{% endblock %}
```

### `tests/fixtures/rust_project/templates/partials/header.html`

```html
<header id="site-header" class="bg-white border-b">
    <div class="container mx-auto px-4 py-4 flex items-center justify-between">
        <h1 class="text-2xl font-bold text-primary">{{ site_name }}</h1>
        <nav class="flex gap-6">
            {% for item in nav_items %}
            <a href="{{ item.url }}" class="{% if item.active %}font-semibold text-primary{% else %}text-gray-600 hover:text-primary{% endif %}">
                {{ item.label }}
            </a>
            {% endfor %}
        </nav>
    </div>
</header>
```

### `tests/fixtures/rust_project/templates/partials/footer.html`

```html
<footer id="site-footer" class="border-t mt-auto py-6 text-center">
    <p class="text-sm text-gray-600">&copy; {{ year }} {{ site_name }}</p>
</footer>
```

### `tests/fixtures/rust_project/templates/partials/history.html`

```html
<aside id="history" class="mt-6 p-4 bg-gray-50 rounded-lg">
    <h3 class="text-lg font-semibold mb-2">History</h3>
    <ul class="space-y-1">
        {% for entry in history %}
        <li class="flex justify-between text-sm">
            <span class="font-mono">{{ entry.expression }}</span>
            <span class="font-bold">= {{ entry.result }}</span>
        </li>
        {% endfor %}
    </ul>
</aside>
```

### `tests/fixtures/rust_project/Cargo.toml`

```toml
[package]
name = "rust_project"
version = "0.1.0"
edition = "2021"

[dependencies]
askama = "0.12"
```

### `tests/fixtures/rust_project/src/templates.rs`

```rust
use askama::Template;

#[derive(Template)]
#[template(path = "calculator.html")]
pub struct CalculatorTemplate<'a> {
    pub site_name: &'a str,
    pub nav_items: Vec<NavItem<'a>>,
    pub year: u32,
    pub operation: &'a str,
    pub a: i32,
    pub b: i32,
    pub selected_op: &'a str,
    pub operations: Vec<OperationOption<'a>>,
    pub result: Option<i32>,
    pub history: Vec<HistoryEntry<'a>>,
}

pub struct NavItem<'a> {
    pub url: &'a str,
    pub label: &'a str,
    pub active: bool,
}

pub struct OperationOption<'a> {
    pub value: &'a str,
    pub label: &'a str,
}

pub struct HistoryEntry<'a> {
    pub expression: &'a str,
    pub result: &'a str,
}
```

---

## Part 8: Expected Output Examples

### CSS Shape Output

```json
{
  "path": "styles/globals.css",
  "theme": [
    {"name": "--color-primary", "value": "oklch(0.6 0.2 250)", "line": 5},
    {"name": "--color-primary-hover", "value": "oklch(0.5 0.25 250)", "line": 6},
    {"name": "--color-secondary", "value": "oklch(0.7 0.15 160)", "line": 7},
    {"name": "--spacing-lg", "value": "1.5rem", "line": 15},
    {"name": "--font-display", "value": "\"Inter\", system-ui, sans-serif", "line": 20}
  ],
  "custom_classes": [
    {"name": "btn", "applied_utilities": ["inline-flex", "items-center", "justify-center", "px-4", "py-2", "rounded-md", "font-medium", "transition-colors"], "layer": "components", "line": 38},
    {"name": "btn-primary", "applied_utilities": ["btn", "bg-primary", "text-white", "hover:bg-primary-hover"], "layer": "components", "line": 42},
    {"name": "btn-secondary", "applied_utilities": ["btn", "bg-secondary", "text-white"], "layer": "components", "line": 46},
    {"name": "card", "applied_utilities": ["bg-white", "rounded-lg", "shadow-md", "p-6"], "layer": "components", "line": 54},
    {"name": "input", "applied_utilities": ["w-full", "px-4", "py-2", "rounded-md", "border", "border-gray-300", "focus:ring-2", "focus:ring-primary"], "layer": "components", "line": 66},
    {"name": "calc-display", "applied_utilities": ["w-full", "p-4", "text-right", "text-3xl", "font-mono", "bg-gray-900", "text-green-400", "rounded-t-lg"], "layer": "components", "line": 74},
    {"name": "animate-fade-in", "applied_utilities": [], "layer": "utilities", "line": 86}
  ],
  "keyframes": [
    {"name": "fade-in", "line": 28},
    {"name": "slide-up", "line": 33}
  ]
}
```

### HTML Shape Output

```json
{
  "path": "index.html",
  "ids": [
    {"tag": "header", "id": "app-header", "line": 10},
    {"tag": "nav", "id": "main-nav", "line": 13},
    {"tag": "main", "id": "app-content", "line": 19},
    {"tag": "section", "id": "basic", "line": 20},
    {"tag": "div", "id": "calc-display", "line": 24},
    {"tag": "div", "id": "calc-keypad", "line": 25},
    {"tag": "button", "id": "clear-btn", "line": 42},
    {"tag": "section", "id": "advanced", "line": 45},
    {"tag": "form", "id": "custom-operation", "line": 49},
    {"tag": "input", "id": "input-a", "line": 52},
    {"tag": "input", "id": "input-b", "line": 56},
    {"tag": "footer", "id": "app-footer", "line": 63}
  ],
  "classes_used": [
    "btn-primary", "btn-secondary", "card", "card-header", "card-title",
    "calc-display", "calc-key", "calc-key-operator", "input", "label"
  ],
  "scripts": [{"src": "./index.js", "inline": false, "line": 6}],
  "styles": [{"href": "./styles/globals.css", "inline": false, "line": 5}]
}
```

### Merged Template Output (merge_templates=true)

```json
{
  "path": "templates/calculator.html",
  "merged_content": "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <title>{{ operation }} - Calculator</title>\n    <link rel=\"stylesheet\" href=\"/static/globals.css\">\n    \n</head>\n<body class=\"min-h-screen bg-gray-50\">\n    <header id=\"site-header\" class=\"bg-white border-b\">...</header>\n    \n    <main id=\"app-content\" class=\"container mx-auto px-4 py-8\">\n        <section id=\"calculator\" class=\"card max-w-md mx-auto\">...</section>\n    </main>\n    \n    <footer id=\"site-footer\" class=\"border-t mt-auto py-6 text-center\">...</footer>\n    \n    \n</body>\n</html>",
  "dependencies": [
    {"path": "base.html", "dependency_type": "extends"},
    {"path": "partials/header.html", "dependency_type": "include"},
    {"path": "partials/footer.html", "dependency_type": "include"},
    {"path": "partials/history.html", "dependency_type": "include"}
  ]
}
```

---

## Part 9: Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/analysis/shape.rs` | Modify | Add `CssFileShape`, `HtmlFileShape`, `ThemeVariable`, `CustomClass`, extraction functions |
| `src/analysis/file_shape.rs` | Modify | Add HTML/CSS shape extraction, `merge_templates` param, Askama resolution |
| `src/analysis/get_context.rs` | Modify | Add HTML/CSS context node types |
| `src/tools.rs` | Modify | Add `merge_templates` parameter to file_shape schema |
| `Cargo.toml` | Modify | Add `regex = "1.10"` dependency |
| `tests/fixtures/typescript_project/index.html` | Create | HTML test fixture |
| `tests/fixtures/typescript_project/styles/globals.css` | Create | Tailwind v4 CSS fixture |
| `tests/fixtures/rust_project/templates/base.html` | Create | Askama base template |
| `tests/fixtures/rust_project/templates/calculator.html` | Create | Askama child template |
| `tests/fixtures/rust_project/templates/partials/header.html` | Create | Askama partial |
| `tests/fixtures/rust_project/templates/partials/footer.html` | Create | Askama partial |
| `tests/fixtures/rust_project/templates/partials/history.html` | Create | Askama partial |
| `tests/fixtures/rust_project/Cargo.toml` | Modify | Add askama dependency |
| `tests/fixtures/rust_project/src/lib.rs` | Modify | Add `pub mod templates;` |
| `tests/fixtures/rust_project/src/templates.rs` | Create | Template structs |
| `tests/html_css_test.rs` | Create | Tests for HTML/CSS extraction |
| `tests/askama_test.rs` | Create | Tests for Askama template merging |

---

## Part 10: Implementation Order

| Phase | Tasks | Priority |
|-------|-------|----------|
| 1 | Add `regex` to Cargo.toml dependencies | High |
| 2 | Add data structures to `shape.rs` | High |
| 3 | Implement `extract_css_tailwind()` with regex | High |
| 4 | Implement `extract_html_shape()` with tree-sitter + filtering | High |
| 5 | Add `merge_templates` param and Askama resolution to `file_shape.rs` | High |
| 6 | Update `get_context.rs` for HTML/CSS nodes | Medium |
| 7 | Update `tools.rs` with new parameter | Medium |
| 8 | Create test fixtures (CSS, HTML, Askama templates) | Medium |
| 9 | Add unit tests for CSS extraction | High |
| 10 | Add unit tests for HTML extraction | High |
| 11 | Add integration tests for Askama merging | High |

---

## Summary

This implementation helps LLMs create consistent UIs by:

1. **Exposing design tokens** from `@theme` block (colors, spacing, fonts, etc.)
2. **Exposing reusable component classes** with their Tailwind utility composition
3. **Tracking custom class usage** in HTML to understand patterns
4. **Providing merged template views** for complete UI structure understanding

The focus is specifically on what helps with UI consistency - not raw Tailwind utilities (which are just noise), but the semantic layer built on top of Tailwind.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use eyre::{bail, Result, WrapErr};
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::debug;
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser, Query, QueryCursor};
use walkdir::{DirEntry, WalkDir};

const HARD_TYPE_LIMIT: usize = 1000;

const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".svelte-kit",
    "target",
    "node_modules",
    "dist",
    "build",
    "deps",
    "vendor",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeExtractionResult {
    pub types: Vec<TypeDefinition>,
    pub total_types: usize,
    pub types_included: usize,
    pub limit_hit: Option<LimitHit>,
    pub truncated: bool,
    /// Word frequency map built during extraction (when count_usages=true)
    #[serde(skip)]
    word_counts: HashMap<String, usize>,
}

impl TypeExtractionResult {
    fn new() -> Self {
        Self {
            types: Vec::new(),
            total_types: 0,
            types_included: 0,
            limit_hit: None,
            truncated: false,
            word_counts: HashMap::new(),
        }
    }

    fn finalize(&mut self) {
        self.types_included = self.types.len();
        self.truncated = self.limit_hit.is_some();
    }

    /// Apply word counts to populate usage_count for each type
    fn apply_usage_counts(&mut self) {
        // Count how many times each type name is defined (to subtract from usage)
        let mut definition_counts: HashMap<String, usize> = HashMap::new();
        for ty in &self.types {
            *definition_counts.entry(ty.name.clone()).or_insert(0) += 1;
        }

        // Apply counts
        for ty in &mut self.types {
            if let Some(&count) = self.word_counts.get(&ty.name) {
                let def_count = definition_counts.get(&ty.name).copied().unwrap_or(1);
                ty.usage_count = count.saturating_sub(def_count);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LimitHit {
    TypeLimit,
    TokenLimit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    Struct,
    Class,
    Enum,
    Trait,
    Interface,
    Protocol,
    TypeAlias,
    Record,
    TypedDict,
    NamedTuple,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub type_annotation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Variant {
    pub name: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_annotation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    #[serde(rename = "type")]
    pub type_annotation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
    pub file: PathBuf,
    pub line: usize,
    pub signature: String,
    pub usage_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<Field>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<Variant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<Member>>,
}

/// Extract types with optional usage counting in a single pass.
pub fn extract_types_with_options(
    path: impl AsRef<Path>,
    pattern: Option<&str>,
    max_types: usize,
    count_usages: bool,
) -> Result<TypeExtractionResult> {
    use crate::analysis::usage_counter::{count_words_in_content, language_for_path};

    let path = path.as_ref();
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }

    let matcher = match pattern {
        Some(pat) => Some(build_globset(pat)?),
        None => None,
    };

    let root_dir = if path.is_file() {
        path.parent()
            .map(PathBuf::from)
            .unwrap_or_else(PathBuf::new)
    } else {
        path.to_path_buf()
    };

    let effective_limit = if max_types == 0 {
        HARD_TYPE_LIMIT
    } else {
        max_types.min(HARD_TYPE_LIMIT)
    };

    let mut result = TypeExtractionResult::new();

    if path.is_file() {
        process_single_file(
            path,
            &root_dir,
            &matcher,
            effective_limit,
            count_usages,
            &mut result,
        )?;
    } else {
        let walker = WalkDir::new(path).into_iter().filter_entry(should_descend);
        for entry in walker {
            let entry = match entry {
                Ok(value) => value,
                Err(err) => {
                    debug!("Skipping entry due to error: {err}");
                    continue;
                }
            };

            if !entry.file_type().is_file() {
                continue;
            }

            if is_hidden(entry.path()) {
                continue;
            }

            let file_path = entry.path();
            let rel_path = relative_path(&root_dir, file_path);
            if let Some(matcher) = matcher.as_ref() {
                if !matcher.is_match(normalize_path(&rel_path)) {
                    continue;
                }
            }

            // Read file once for both word counting and type extraction
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(err) => {
                    debug!("Skipping file {}: {err}", file_path.display());
                    continue;
                }
            };

            // Count words for usage tracking (works on all file types)
            if count_usages {
                let lang = language_for_path(file_path);
                count_words_in_content(&content, lang, &mut result.word_counts);
            }

            // Extract types from supported languages
            if let Some(language) = detect_language(file_path) {
                if let Err(err) = process_file_with_source(
                    &content,
                    &rel_path,
                    language,
                    effective_limit,
                    &mut result,
                ) {
                    debug!("Skipping file {}: {err}", file_path.display());
                }
            }

            if result.limit_hit.is_some() {
                break;
            }
        }
    }

    result.finalize();

    // Apply usage counts after all files are processed
    if count_usages {
        result.apply_usage_counts();
    }

    Ok(result)
}

fn process_single_file(
    path: &Path,
    root_dir: &Path,
    matcher: &Option<GlobSet>,
    limit: usize,
    count_usages: bool,
    result: &mut TypeExtractionResult,
) -> Result<()> {
    use crate::analysis::usage_counter::{count_words_in_content, language_for_path};

    if is_hidden(path) {
        return Ok(());
    }

    let rel_path = relative_path(root_dir, path);
    if let Some(matcher) = matcher.as_ref() {
        if !matcher.is_match(normalize_path(&rel_path)) {
            return Ok(());
        }
    }

    // Read file once for both extraction and counting
    let source =
        fs::read_to_string(path).wrap_err_with(|| format!("Failed to read {}", path.display()))?;

    // Count words if requested
    if count_usages {
        let lang = language_for_path(path);
        count_words_in_content(&source, lang, &mut result.word_counts);
    }

    if let Some(language) = detect_language(path) {
        process_file_with_source(&source, &rel_path, language, limit, result)?;
    }
    Ok(())
}

fn process_file_with_source(
    source: &str,
    relative_path: &Path,
    language: SupportedLanguage,
    limit: usize,
    result: &mut TypeExtractionResult,
) -> Result<()> {
    let file_types = match language {
        SupportedLanguage::Rust => extract_rust_types(source, relative_path)?,
        SupportedLanguage::TypeScript => extract_typescript_types(source, relative_path, true)?,
        SupportedLanguage::JavaScript => extract_typescript_types(source, relative_path, false)?,
        SupportedLanguage::Python => extract_python_types(source, relative_path)?,
        SupportedLanguage::Java => extract_java_types(source, relative_path)?,
        SupportedLanguage::CSharp => extract_csharp_types(source, relative_path)?,
        SupportedLanguage::Go => extract_go_types(source, relative_path)?,
    };

    for ty in file_types {
        result.total_types += 1;
        if result.types.len() < limit {
            result.types.push(ty);
        } else {
            result.limit_hit = Some(LimitHit::TypeLimit);
            break;
        }
    }

    Ok(())
}

fn build_globset(pattern: &str) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    builder.add(Glob::new(pattern)?);
    builder.build().map_err(Into::into)
}

fn should_descend(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    if !entry.file_type().is_dir() {
        return true;
    }

    let Some(name) = entry.file_name().to_str() else {
        return false;
    };

    if name.starts_with('.') {
        return false;
    }

    !IGNORED_DIR_NAMES
        .iter()
        .any(|skip| skip.eq_ignore_ascii_case(name))
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn relative_path(root: &Path, file: &Path) -> PathBuf {
    if root.as_os_str().is_empty() {
        return file.to_path_buf();
    }

    if let Ok(relative) = file.strip_prefix(root) {
        if relative.as_os_str().is_empty() {
            if let Some(name) = file.file_name() {
                return PathBuf::from(name);
            }
        } else {
            return relative.to_path_buf();
        }
    }

    file.to_path_buf()
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[derive(Debug, Clone, Copy)]
enum SupportedLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Java,
    CSharp,
    Go,
}

fn detect_language(path: &Path) -> Option<SupportedLanguage> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some(SupportedLanguage::Rust),
        "ts" | "tsx" => Some(SupportedLanguage::TypeScript),
        "js" | "jsx" | "mjs" | "cjs" => Some(SupportedLanguage::JavaScript),
        "py" => Some(SupportedLanguage::Python),
        "java" => Some(SupportedLanguage::Java),
        "cs" => Some(SupportedLanguage::CSharp),
        "go" => Some(SupportedLanguage::Go),
        _ => None,
    }
}

pub(crate) fn extract_rust_types(
    source: &str,
    relative_path: &Path,
) -> Result<Vec<TypeDefinition>> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .wrap_err("Failed to configure Rust parser")?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| eyre::eyre!("Failed to parse Rust source"))?;

    let query_src = r#"
        (struct_item name: (type_identifier) @name) @struct
        (enum_item name: (type_identifier) @name) @enum
        (trait_item name: (type_identifier) @name) @trait
        (type_item name: (type_identifier) @name) @alias
        (impl_item type: (type_identifier) @impl_name) @impl
    "#;

    let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), query_src)
        .wrap_err("Failed to compile Rust query")?;

    let mut type_map: HashMap<String, TypeDefinition> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let source_bytes = source.as_bytes();
    let file_path = relative_path.to_path_buf();

    let mut cursor = QueryCursor::new();
    for match_ in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut type_name = String::new();
        let mut kind = TypeKind::TypeAlias;
        let mut def_node = None;
        let mut is_impl = false;

        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => {
                    type_name = capture
                        .node
                        .utf8_text(source_bytes)
                        .unwrap_or_default()
                        .to_string();
                }
                "struct" => {
                    kind = TypeKind::Struct;
                    def_node = Some(capture.node);
                }
                "enum" => {
                    kind = TypeKind::Enum;
                    def_node = Some(capture.node);
                }
                "trait" => {
                    kind = TypeKind::Trait;
                    def_node = Some(capture.node);
                }
                "alias" => {
                    kind = TypeKind::TypeAlias;
                    def_node = Some(capture.node);
                }
                "impl" | "impl_name" => {
                    is_impl = true;
                }
                _ => {}
            }
        }

        if is_impl || type_name.is_empty() {
            continue;
        }

        let Some(node) = def_node else {
            continue;
        };

        if type_map.contains_key(&type_name) {
            continue;
        }

        let mut def = TypeDefinition {
            name: type_name.clone(),
            kind,
            file: file_path.clone(),
            line: node.start_position().row + 1,
            signature: signature_for(node, source_bytes),
            usage_count: 0,
            fields: None,
            variants: None,
            members: None,
        };

        match kind {
            TypeKind::Struct => {
                if let Some(body) = node.child_by_field_name("body") {
                    let mut struct_fields = Vec::new();
                    let mut walker = body.walk();
                    for child in body.children(&mut walker) {
                        if child.kind() == "field_declaration" {
                            let name = child
                                .child_by_field_name("name")
                                .and_then(|n| n.utf8_text(source_bytes).ok())
                                .unwrap_or_default()
                                .to_string();
                            if name.is_empty() {
                                continue;
                            }
                            let type_annotation = child
                                .child_by_field_name("type")
                                .and_then(|n| n.utf8_text(source_bytes).ok())
                                .unwrap_or_default()
                                .to_string();
                            struct_fields.push(Field {
                                name,
                                type_annotation,
                            });
                        }
                    }
                    if !struct_fields.is_empty() {
                        def.fields = Some(struct_fields);
                    }
                }
            }
            TypeKind::Enum => {
                if let Some(body) = node.child_by_field_name("body") {
                    let mut enum_variants = Vec::new();
                    let mut walker = body.walk();
                    for child in body.children(&mut walker) {
                        if child.kind() == "enum_variant" {
                            let name = child
                                .child_by_field_name("name")
                                .and_then(|n| n.utf8_text(source_bytes).ok())
                                .unwrap_or_default()
                                .to_string();
                            if name.is_empty() {
                                continue;
                            }
                            enum_variants.push(Variant {
                                name,
                                type_annotation: None,
                            });
                        }
                    }
                    if !enum_variants.is_empty() {
                        def.variants = Some(enum_variants);
                    }
                }
            }
            TypeKind::Trait => {
                if let Some(body) = node.child_by_field_name("body") {
                    let mut members = Vec::new();
                    let mut walker = body.walk();
                    for child in body.children(&mut walker) {
                        if matches!(
                            child.kind(),
                            "associated_type_declaration"
                                | "associated_type"
                                | "associated_type_item"
                        ) {
                            let name = child
                                .child_by_field_name("name")
                                .and_then(|n| n.utf8_text(source_bytes).ok())
                                .or_else(|| {
                                    let mut walker = child.walk();
                                    let found = child
                                        .children(&mut walker)
                                        .find(|n| n.kind() == "type_identifier")
                                        .and_then(|n| n.utf8_text(source_bytes).ok());
                                    found
                                })
                                .unwrap_or_default()
                                .to_string();
                            if name.is_empty() {
                                continue;
                            }
                            members.push(Member {
                                name,
                                type_annotation: "associated_type".to_string(),
                            });
                            continue;
                        }

                        if child.kind() == "function_item"
                            || child.kind() == "function_signature_item"
                        {
                            let name = child
                                .child_by_field_name("name")
                                .and_then(|n| n.utf8_text(source_bytes).ok())
                                .unwrap_or_default()
                                .to_string();
                            if name.is_empty() {
                                continue;
                            }
                            members.push(Member {
                                name,
                                type_annotation: signature_for(child, source_bytes),
                            });
                        }
                    }
                    if !members.is_empty() {
                        def.members = Some(members);
                    }
                }
            }
            _ => {}
        }

        order.push(type_name.clone());
        type_map.insert(type_name, def);
    }

    let mut cursor = QueryCursor::new();
    for match_ in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut type_name = String::new();
        let mut impl_node = None;
        let mut is_impl = false;

        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            match capture_name {
                "impl_name" => {
                    type_name = capture
                        .node
                        .utf8_text(source_bytes)
                        .unwrap_or_default()
                        .to_string();
                }
                "impl" => {
                    is_impl = true;
                    impl_node = Some(capture.node);
                }
                _ => {}
            }
        }

        if !is_impl || type_name.is_empty() {
            continue;
        }

        let Some(definition) = type_map.get_mut(&type_name) else {
            continue;
        };
        let Some(node) = impl_node else {
            continue;
        };
        let Some(body) = node.child_by_field_name("body") else {
            continue;
        };

        let mut members = definition.members.take().unwrap_or_default();
        let mut walker = body.walk();
        for child in body.children(&mut walker) {
            if child.kind() == "function_item" {
                let name = child
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source_bytes).ok())
                    .unwrap_or_default()
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                members.push(Member {
                    name,
                    type_annotation: signature_for(child, source_bytes),
                });
            }
        }
        if !members.is_empty() {
            definition.members = Some(members);
        }
    }

    let mut definitions = Vec::new();
    for name in order {
        if let Some(def) = type_map.remove(&name) {
            definitions.push(def);
        }
    }

    Ok(definitions)
}

pub(crate) fn extract_typescript_types(
    source: &str,
    relative_path: &Path,
    is_typescript: bool,
) -> Result<Vec<TypeDefinition>> {
    let mut parser = Parser::new();
    let language = if is_typescript {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    } else {
        tree_sitter_javascript::LANGUAGE.into()
    };
    parser
        .set_language(&language)
        .wrap_err("Failed to configure TypeScript parser")?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| eyre::eyre!("Failed to parse TypeScript source"))?;

    let query_src = if is_typescript {
        r#"
        (class_declaration name: (type_identifier) @name) @class
        (interface_declaration name: (type_identifier) @name) @interface
        (type_alias_declaration name: (type_identifier) @name) @alias
        (enum_declaration name: (identifier) @name) @enum
        "#
    } else {
        r#"
        (class_declaration name: (identifier) @name) @class
        "#
    };

    let query = Query::new(&language, query_src).wrap_err("Failed to compile TypeScript query")?;
    let source_bytes = source.as_bytes();
    let file_path = relative_path.to_path_buf();
    let mut definitions = Vec::new();
    let mut cursor = QueryCursor::new();

    for match_ in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut name_node = None;
        let mut def_node = None;
        let mut kind = TypeKind::TypeAlias;

        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => name_node = Some(capture.node),
                "class" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Class;
                }
                "interface" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Interface;
                }
                "alias" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::TypeAlias;
                }
                "enum" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Enum;
                }
                _ => {}
            }
        }

        let Some(name_node) = name_node else {
            continue;
        };
        let Some(def_node) = def_node else {
            continue;
        };
        let Ok(name) = name_node.utf8_text(source_bytes) else {
            continue;
        };

        let mut fields = None;
        let mut members = None;
        let mut variants = None;

        match kind {
            TypeKind::Class => {
                fields = collect_ts_fields(def_node, source_bytes);
            }
            TypeKind::Interface => {
                members = collect_ts_members(def_node, source_bytes);
            }
            TypeKind::Enum => {
                variants = collect_ts_variants(def_node, source_bytes);
            }
            _ => {}
        }

        definitions.push(TypeDefinition {
            name: name.to_string(),
            kind,
            file: file_path.clone(),
            line: def_node.start_position().row + 1,
            signature: signature_for(def_node, source_bytes),
            usage_count: 0,
            fields,
            variants,
            members,
        });
    }

    Ok(definitions)
}

fn collect_ts_fields(node: Node, source: &[u8]) -> Option<Vec<Field>> {
    let body = node.child_by_field_name("body")?;
    let mut fields = Vec::new();
    let mut walker = body.walk();
    for child in body.children(&mut walker) {
        if matches!(
            child.kind(),
            "public_field_definition" | "field_definition" | "property_signature"
        ) {
            let name = child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source).ok())
                .unwrap_or_default()
                .to_string();
            if name.is_empty() {
                continue;
            }

            let type_annotation = child
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(source).ok())
                .map(clean_type_annotation)
                .unwrap_or_default();

            fields.push(Field {
                name,
                type_annotation,
            });
        }
    }

    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

fn collect_ts_members(node: Node, source: &[u8]) -> Option<Vec<Member>> {
    let body = node.child_by_field_name("body")?;
    let mut members = Vec::new();
    let mut walker = body.walk();
    for child in body.children(&mut walker) {
        if matches!(child.kind(), "property_signature" | "method_signature") {
            let name = child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source).ok())
                .unwrap_or_default()
                .to_string();
            if name.is_empty() {
                continue;
            }

            let type_annotation = if child.kind() == "property_signature" {
                child
                    .child_by_field_name("type")
                    .and_then(|n| n.utf8_text(source).ok())
                    .map(clean_type_annotation)
                    .unwrap_or_default()
            } else {
                signature_for(child, source)
            };

            members.push(Member {
                name,
                type_annotation,
            });
        }
    }

    if members.is_empty() {
        None
    } else {
        Some(members)
    }
}

fn collect_ts_variants(node: Node, source: &[u8]) -> Option<Vec<Variant>> {
    let body = node.child_by_field_name("body")?;
    let mut variants = Vec::new();
    let mut walker = body.walk();
    for child in body.children(&mut walker) {
        if child.kind() == "enum_member" {
            let name = child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source).ok())
                .unwrap_or_default()
                .to_string();
            if name.is_empty() {
                continue;
            }
            variants.push(Variant {
                name,
                type_annotation: None,
            });
        }
    }

    if variants.is_empty() {
        None
    } else {
        Some(variants)
    }
}

pub(crate) fn extract_python_types(
    source: &str,
    relative_path: &Path,
) -> Result<Vec<TypeDefinition>> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .wrap_err("Failed to configure Python parser")?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| eyre::eyre!("Failed to parse Python source"))?;

    let query_src = r#"
        (class_definition name: (identifier) @name) @class
        (assignment
            left: (identifier) @name
            right: (call
                function: (identifier) @func
                arguments: (argument_list) @args)
        ) @special_assignment
    "#;

    let query = Query::new(&tree_sitter_python::LANGUAGE.into(), query_src)
        .wrap_err("Failed to compile Python query")?;

    let source_bytes = source.as_bytes();
    let file_path = relative_path.to_path_buf();
    let mut definitions = Vec::new();
    let mut cursor = QueryCursor::new();

    for match_ in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut name_node = None;
        let mut def_node = None;
        let mut is_special = false;
        let mut func_name = String::new();

        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => name_node = Some(capture.node),
                "class" => def_node = Some(capture.node),
                "special_assignment" => {
                    def_node = Some(capture.node);
                    is_special = true;
                }
                "func" => {
                    func_name = capture
                        .node
                        .utf8_text(source_bytes)
                        .unwrap_or_default()
                        .to_string();
                }
                _ => {}
            }
        }

        let Some(name_node) = name_node else {
            continue;
        };
        let Some(def_node) = def_node else {
            continue;
        };
        let Ok(name) = name_node.utf8_text(source_bytes) else {
            continue;
        };

        if is_special {
            let kind = match func_name.as_str() {
                "TypedDict" => TypeKind::TypedDict,
                "NamedTuple" => TypeKind::NamedTuple,
                _ => continue,
            };

            let fields = if let Some(args) = def_node
                .child_by_field_name("right")
                .and_then(|right| right.child_by_field_name("arguments"))
            {
                match kind {
                    TypeKind::TypedDict => parse_python_typed_dict_fields(args, source_bytes),
                    TypeKind::NamedTuple => parse_python_named_tuple_fields(args, source_bytes),
                    _ => None,
                }
            } else {
                None
            };

            definitions.push(TypeDefinition {
                name: name.to_string(),
                kind,
                file: file_path.clone(),
                line: def_node.start_position().row + 1,
                signature: signature_for(def_node, source_bytes),
                usage_count: 0,
                fields,
                variants: None,
                members: None,
            });
            continue;
        }

        let mut kind = TypeKind::Class;
        let mut fields = Vec::new();
        let mut variants = None;
        let mut members = None;

        // Check if it's an Enum or Protocol
        if let Some(superclasses) = def_node.child_by_field_name("superclasses") {
            let text = superclasses.utf8_text(source_bytes).unwrap_or_default();
            if text.contains("Enum") {
                kind = TypeKind::Enum;
            } else if text.contains("Protocol") {
                kind = TypeKind::Protocol;
            }
        }

        if let Some(body) = def_node.child_by_field_name("body") {
            let mut walker = body.walk();
            for child in body.children(&mut walker) {
                match child.kind() {
                    "function_definition" => {
                        let fname = child
                            .child_by_field_name("name")
                            .and_then(|n| n.utf8_text(source_bytes).ok())
                            .unwrap_or_default();
                        if fname == "__init__" {
                            // Extract fields from self assignments
                            if let Some(fbody) = child.child_by_field_name("body") {
                                let mut fwalker = fbody.walk();
                                for stmt in fbody.children(&mut fwalker) {
                                    if stmt.kind() == "expression_statement" {
                                        if let Some(assignment) = stmt.child(0) {
                                            if assignment.kind() == "assignment" {
                                                if let Some(left) =
                                                    assignment.child_by_field_name("left")
                                                {
                                                    if left.kind() == "attribute" {
                                                        if let Some(obj) =
                                                            left.child_by_field_name("object")
                                                        {
                                                            if obj
                                                                .utf8_text(source_bytes)
                                                                .unwrap_or_default()
                                                                == "self"
                                                            {
                                                                if let Some(attr) = left
                                                                    .child_by_field_name(
                                                                        "attribute",
                                                                    )
                                                                {
                                                                    let field_name = attr
                                                                        .utf8_text(source_bytes)
                                                                        .unwrap_or_default()
                                                                        .to_string();
                                                                    fields.push(Field {
                                                                        name: field_name,
                                                                        type_annotation: "Any"
                                                                            .to_string(),
                                                                    });
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Track methods for all classes/protocols (legacy type_map expects this)
                        let m = members.get_or_insert_with(Vec::new);
                        m.push(Member {
                            name: fname.to_string(),
                            type_annotation: signature_for(child, source_bytes),
                        });
                    }
                    "decorated_definition" => {
                        let definition = child.child_by_field_name("definition").or_else(|| {
                            let mut dwalker = child.walk();
                            let found = child
                                .children(&mut dwalker)
                                .find(|n| n.kind() == "function_definition");
                            found
                        });

                        let Some(definition) = definition else {
                            continue;
                        };

                        let fname = definition
                            .child_by_field_name("name")
                            .and_then(|n| n.utf8_text(source_bytes).ok())
                            .unwrap_or_default();

                        if !fname.is_empty() {
                            let m = members.get_or_insert_with(Vec::new);
                            m.push(Member {
                                name: fname.to_string(),
                                type_annotation: signature_for(definition, source_bytes),
                            });
                        }
                    }
                    "expression_statement" => {
                        if let Some(statement) = child.child(0) {
                            if statement.kind() == "assignment" {
                                if kind == TypeKind::Enum {
                                    let v = variants.get_or_insert_with(Vec::new);
                                    if let Some(left) = statement.child_by_field_name("left") {
                                        let vname =
                                            left.utf8_text(source_bytes).unwrap_or_default();
                                        if !vname.is_empty() {
                                            v.push(Variant {
                                                name: vname.to_string(),
                                                type_annotation: None,
                                            });
                                        }
                                    }
                                    continue;
                                }

                                // Protocol: treat typed class attributes as members
                                if kind == TypeKind::Protocol {
                                    if let Some(left) = statement.child_by_field_name("left") {
                                        if left.kind() == "identifier" {
                                            let name = left
                                                .utf8_text(source_bytes)
                                                .unwrap_or_default()
                                                .to_string();
                                            if !name.is_empty() {
                                                let type_annotation = statement
                                                    .child_by_field_name("type")
                                                    .and_then(|n| n.utf8_text(source_bytes).ok())
                                                    .unwrap_or("Any")
                                                    .to_string();
                                                let m = members.get_or_insert_with(Vec::new);
                                                m.push(Member {
                                                    name,
                                                    type_annotation,
                                                });
                                            }
                                        }
                                    }
                                    continue;
                                }

                                // Class attributes / fields
                                if let Some(left) = statement.child_by_field_name("left") {
                                    if left.kind() == "identifier" {
                                        let name = left
                                            .utf8_text(source_bytes)
                                            .unwrap_or_default()
                                            .to_string();
                                        if !name.is_empty() {
                                            let type_annotation = statement
                                                .child_by_field_name("type")
                                                .and_then(|n| n.utf8_text(source_bytes).ok())
                                                .unwrap_or("Any")
                                                .to_string();
                                            fields.push(Field {
                                                name,
                                                type_annotation,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        definitions.push(TypeDefinition {
            name: name.to_string(),
            kind,
            file: file_path.clone(),
            line: def_node.start_position().row + 1,
            signature: signature_for(def_node, source_bytes),
            usage_count: 0,
            fields: if fields.is_empty() {
                None
            } else {
                Some(fields)
            },
            variants,
            members,
        });
    }

    Ok(definitions)
}

fn parse_python_typed_dict_fields(args: Node, source: &[u8]) -> Option<Vec<Field>> {
    let mut walker = args.walk();
    let dict_node = args
        .children(&mut walker)
        .find(|child| child.kind() == "dictionary")?;

    let mut fields = Vec::new();
    let mut dict_walker = dict_node.walk();
    for pair in dict_node.children(&mut dict_walker) {
        if pair.kind() != "pair" {
            continue;
        }

        let key_node = pair.child_by_field_name("key").or_else(|| pair.child(0))?;
        let value_node = pair
            .child_by_field_name("value")
            .or_else(|| pair.child(pair.child_count().saturating_sub(1)))?;

        let raw_key = key_node.utf8_text(source).ok()?;
        let name = unquote_python_string(raw_key);
        if name.is_empty() {
            continue;
        }

        let type_annotation = value_node.utf8_text(source).unwrap_or("Any").to_string();

        fields.push(Field {
            name,
            type_annotation,
        });
    }

    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

fn parse_python_named_tuple_fields(args: Node, source: &[u8]) -> Option<Vec<Field>> {
    let mut walker = args.walk();
    let list_node = args
        .children(&mut walker)
        .find(|child| child.kind() == "list")?;

    let mut fields = Vec::new();
    let mut list_walker = list_node.walk();
    for tuple_node in list_node.children(&mut list_walker) {
        if tuple_node.kind() != "tuple" {
            continue;
        }

        let mut tuple_walker = tuple_node.walk();
        let mut items: Vec<Node> = tuple_node
            .children(&mut tuple_walker)
            .filter(|n| n.is_named())
            .collect();
        if items.len() < 2 {
            continue;
        }

        let raw_name = items.remove(0).utf8_text(source).ok()?;
        let name = unquote_python_string(raw_name);
        if name.is_empty() {
            continue;
        }

        let type_annotation = items[0].utf8_text(source).unwrap_or("Any").to_string();

        fields.push(Field {
            name,
            type_annotation,
        });
    }

    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

fn unquote_python_string(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let first = bytes[0] as char;
        let last = bytes[trimmed.len() - 1] as char;
        if (first == '\'' && last == '\'') || (first == '"' && last == '"') {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn extract_java_types(source: &str, relative_path: &Path) -> Result<Vec<TypeDefinition>> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .wrap_err("Failed to configure Java parser")?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| eyre::eyre!("Failed to parse Java source"))?;

    let query_src = r#"
        (class_declaration name: (identifier) @name) @class
        (interface_declaration name: (identifier) @name) @interface
        (enum_declaration name: (identifier) @name) @enum
        (record_declaration name: (identifier) @name) @record
    "#;

    let query = Query::new(&tree_sitter_java::LANGUAGE.into(), query_src)
        .wrap_err("Failed to compile Java query")?;

    let source_bytes = source.as_bytes();
    let file_path = relative_path.to_path_buf();
    let mut definitions = Vec::new();
    let mut cursor = QueryCursor::new();

    for match_ in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut name_node = None;
        let mut def_node = None;
        let mut kind = TypeKind::Class;

        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => name_node = Some(capture.node),
                "class" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Class;
                }
                "interface" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Interface;
                }
                "enum" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Enum;
                }
                "record" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Record;
                }
                _ => {}
            }
        }

        let Some(name_node) = name_node else {
            continue;
        };
        let Some(def_node) = def_node else {
            continue;
        };
        let Ok(name) = name_node.utf8_text(source_bytes) else {
            continue;
        };

        let mut fields = None;
        let mut members = None;
        let mut variants = None;

        if let Some(body) = def_node.child_by_field_name("body") {
            let mut f = Vec::new();
            let mut m = Vec::new();
            let mut v = Vec::new();
            let mut walker = body.walk();

            for child in body.children(&mut walker) {
                match child.kind() {
                    "field_declaration" => {
                        if let Some(type_node) = child.child_by_field_name("type") {
                            let type_str = type_node.utf8_text(source_bytes).unwrap_or_default();
                            let mut dwalker = child.walk();
                            for d in child.children(&mut dwalker) {
                                if d.kind() == "variable_declarator" {
                                    if let Some(n) = d.child_by_field_name("name") {
                                        f.push(Field {
                                            name: n
                                                .utf8_text(source_bytes)
                                                .unwrap_or_default()
                                                .to_string(),
                                            type_annotation: type_str.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    "method_declaration" if kind == TypeKind::Interface => {
                        if let Some(n) = child.child_by_field_name("name") {
                            m.push(Member {
                                name: n.utf8_text(source_bytes).unwrap_or_default().to_string(),
                                type_annotation: signature_for(child, source_bytes),
                            });
                        }
                    }
                    "enum_constant" => {
                        if let Some(n) = child.child_by_field_name("name") {
                            v.push(Variant {
                                name: n.utf8_text(source_bytes).unwrap_or_default().to_string(),
                                type_annotation: None,
                            });
                        }
                    }
                    _ => {}
                }
            }
            if !f.is_empty() {
                fields = Some(f);
            }
            if !m.is_empty() {
                members = Some(m);
            }
            if !v.is_empty() {
                variants = Some(v);
            }
        }

        definitions.push(TypeDefinition {
            name: name.to_string(),
            kind,
            file: file_path.clone(),
            line: def_node.start_position().row + 1,
            signature: signature_for(def_node, source_bytes),
            usage_count: 0,
            fields,
            variants,
            members,
        });
    }

    Ok(definitions)
}

fn extract_go_types(source: &str, relative_path: &Path) -> Result<Vec<TypeDefinition>> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .wrap_err("Failed to configure Go parser")?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| eyre::eyre!("Failed to parse Go source"))?;

    let query_src = r#"
        (type_spec name: (type_identifier) @name type: (struct_type) @struct) @struct_spec
        (type_spec name: (type_identifier) @name type: (interface_type) @iface) @iface_spec
    "#;

    let query = Query::new(&tree_sitter_go::LANGUAGE.into(), query_src)
        .wrap_err("Failed to compile Go query")?;

    let source_bytes = source.as_bytes();
    let file_path = relative_path.to_path_buf();
    let mut definitions = Vec::new();
    let mut cursor = QueryCursor::new();

    for match_ in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut name_node = None;
        let mut def_node = None;
        let mut kind = TypeKind::Struct;
        let mut struct_node = None;
        let mut iface_node = None;

        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => name_node = Some(capture.node),
                "struct_spec" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Struct;
                }
                "iface_spec" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Interface;
                }
                "struct" => struct_node = Some(capture.node),
                "iface" => iface_node = Some(capture.node),
                _ => {}
            }
        }

        let Some(name_node) = name_node else {
            continue;
        };
        let Some(def_node) = def_node else {
            continue;
        };
        let Ok(name) = name_node.utf8_text(source_bytes) else {
            continue;
        };

        let mut def = TypeDefinition {
            name: name.to_string(),
            kind,
            file: file_path.clone(),
            line: def_node.start_position().row + 1,
            signature: signature_for(def_node, source_bytes),
            usage_count: 0,
            fields: None,
            variants: None,
            members: None,
        };

        match kind {
            TypeKind::Struct => {
                let Some(struct_node) = struct_node else {
                    definitions.push(def);
                    continue;
                };

                let fields_list =
                    if let Some(fields_list) = struct_node.child_by_field_name("fields") {
                        Some(fields_list)
                    } else {
                        let mut walker = struct_node.walk();
                        let mut children = struct_node.children(&mut walker);
                        children.find(|n| n.kind() == "field_declaration_list")
                    };

                let Some(fields_list) = fields_list else {
                    definitions.push(def);
                    continue;
                };

                let mut fields = Vec::new();
                let mut walker = fields_list.walk();
                for field_decl in fields_list.children(&mut walker) {
                    if field_decl.kind() != "field_declaration" {
                        continue;
                    }

                    let type_annotation = field_decl
                        .child_by_field_name("type")
                        .and_then(|t| t.utf8_text(source_bytes).ok())
                        .unwrap_or_default()
                        .to_string();

                    if type_annotation.is_empty() {
                        continue;
                    }

                    let mut name_walker = field_decl.walk();
                    let names: Vec<String> = field_decl
                        .children(&mut name_walker)
                        .filter(|n| n.kind() == "field_identifier")
                        .filter_map(|n| n.utf8_text(source_bytes).ok().map(|s| s.to_string()))
                        .collect();

                    for name in names {
                        if !name.is_empty() {
                            fields.push(Field {
                                name,
                                type_annotation: type_annotation.clone(),
                            });
                        }
                    }
                }

                if !fields.is_empty() {
                    def.fields = Some(fields);
                }
            }
            TypeKind::Interface => {
                let Some(iface_node) = iface_node else {
                    definitions.push(def);
                    continue;
                };

                let mut members = Vec::new();
                let mut stack = vec![iface_node];
                while let Some(node) = stack.pop() {
                    let mut walker = node.walk();
                    for child in node.named_children(&mut walker) {
                        if child.kind() == "method_elem" {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                let name = name_node
                                    .utf8_text(source_bytes)
                                    .unwrap_or_default()
                                    .to_string();
                                if !name.is_empty() {
                                    members.push(Member {
                                        name,
                                        type_annotation: signature_for(child, source_bytes),
                                    });
                                }
                            }
                        }
                        stack.push(child);
                    }
                }

                if !members.is_empty() {
                    def.members = Some(members);
                }
            }
            _ => {}
        }

        definitions.push(def);
    }

    Ok(definitions)
}

fn extract_csharp_types(source: &str, relative_path: &Path) -> Result<Vec<TypeDefinition>> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .wrap_err("Failed to configure C# parser")?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| eyre::eyre!("Failed to parse C# source"))?;

    let query_src = r#"
        (class_declaration name: (identifier) @name) @class
        (interface_declaration name: (identifier) @name) @interface
        (struct_declaration name: (identifier) @name) @struct
        (enum_declaration name: (identifier) @name) @enum
        (record_declaration name: (identifier) @name) @record
        (using_directive (name_equals (identifier) @name)) @alias
    "#;

    // Some versions of tree-sitter-c-sharp have different `using` alias shapes.
    // If the alias query doesn't compile, fall back to the core type patterns.
    let query_src_fallback = r#"
        (class_declaration name: (identifier) @name) @class
        (interface_declaration name: (identifier) @name) @interface
        (struct_declaration name: (identifier) @name) @struct
        (enum_declaration name: (identifier) @name) @enum
        (record_declaration name: (identifier) @name) @record
    "#;

    let query = Query::new(&tree_sitter_c_sharp::LANGUAGE.into(), query_src)
        .or_else(|_| Query::new(&tree_sitter_c_sharp::LANGUAGE.into(), query_src_fallback))
        .wrap_err("Failed to compile C# query")?;

    let source_bytes = source.as_bytes();
    let file_path = relative_path.to_path_buf();
    let mut definitions = Vec::new();
    let mut cursor = QueryCursor::new();

    for match_ in cursor.matches(&query, tree.root_node(), source_bytes) {
        let mut name_node = None;
        let mut def_node = None;
        let mut kind = TypeKind::Class;
        let mut is_alias = false;

        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => name_node = Some(capture.node),
                "class" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Class;
                }
                "interface" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Interface;
                }
                "struct" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Struct;
                }
                "enum" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Enum;
                }
                "record" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::Record;
                }
                "alias" => {
                    def_node = Some(capture.node);
                    kind = TypeKind::TypeAlias;
                    is_alias = true;
                }
                _ => {}
            }
        }

        let Some(name_node) = name_node else {
            continue;
        };
        let Some(def_node) = def_node else {
            continue;
        };
        let Ok(name) = name_node.utf8_text(source_bytes) else {
            continue;
        };

        if is_alias {
            definitions.push(TypeDefinition {
                name: name.to_string(),
                kind,
                file: file_path.clone(),
                line: def_node.start_position().row + 1,
                signature: signature_for(def_node, source_bytes),
                usage_count: 0,
                fields: None,
                variants: None,
                members: None,
            });
            continue;
        }

        let mut fields = None;
        let mut members = None;
        let mut variants = None;

        if let Some(body) = def_node.child_by_field_name("body") {
            let mut f = Vec::new();
            let mut m = Vec::new();
            let mut v = Vec::new();
            let mut walker = body.walk();

            for child in body.children(&mut walker) {
                match child.kind() {
                    "field_declaration" => {
                        if let Some(type_node) = child.child_by_field_name("type") {
                            let type_str = type_node.utf8_text(source_bytes).unwrap_or_default();
                            if let Some(declarators) = child.child_by_field_name("declarators") {
                                let mut dwalker = declarators.walk();
                                for d in declarators.children(&mut dwalker) {
                                    if d.kind() == "variable_declarator" {
                                        if let Some(n) = d.child_by_field_name("name") {
                                            f.push(Field {
                                                name: n
                                                    .utf8_text(source_bytes)
                                                    .unwrap_or_default()
                                                    .to_string(),
                                                type_annotation: type_str.to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "property_declaration" => {
                        if let Some(n) = child.child_by_field_name("name") {
                            let type_ann = child
                                .child_by_field_name("type")
                                .and_then(|t| t.utf8_text(source_bytes).ok())
                                .unwrap_or_default();
                            f.push(Field {
                                name: n.utf8_text(source_bytes).unwrap_or_default().to_string(),
                                type_annotation: type_ann.to_string(),
                            });
                        }
                    }
                    "method_declaration" if kind == TypeKind::Interface => {
                        if let Some(n) = child.child_by_field_name("name") {
                            m.push(Member {
                                name: n.utf8_text(source_bytes).unwrap_or_default().to_string(),
                                type_annotation: signature_for(child, source_bytes),
                            });
                        }
                    }
                    "enum_member_declaration" => {
                        if let Some(n) = child.child_by_field_name("name") {
                            v.push(Variant {
                                name: n.utf8_text(source_bytes).unwrap_or_default().to_string(),
                                type_annotation: None,
                            });
                        }
                    }
                    _ => {}
                }
            }
            if !f.is_empty() {
                fields = Some(f);
            }
            if !m.is_empty() {
                members = Some(m);
            }
            if !v.is_empty() {
                variants = Some(v);
            }
        }

        definitions.push(TypeDefinition {
            name: name.to_string(),
            kind,
            file: file_path.clone(),
            line: def_node.start_position().row + 1,
            signature: signature_for(def_node, source_bytes),
            usage_count: 0,
            fields,
            variants,
            members,
        });
    }

    Ok(definitions)
}

fn signature_for(node: Node, source: &[u8]) -> String {
    if let Ok(text) = node.utf8_text(source) {
        text.lines().next().unwrap_or("").trim().to_string()
    } else {
        String::new()
    }
}

fn clean_type_annotation(text: &str) -> String {
    text.trim()
        .trim_start_matches(':')
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string()
}

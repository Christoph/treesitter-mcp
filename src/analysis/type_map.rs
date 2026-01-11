use std::path::Path;

use eyre::Result;
use serde_json::{json, Value};
use tiktoken_rs::cl100k_base;

use crate::analysis::path_utils;
use crate::analysis::usage_counter::count_all_usages;
use crate::extraction::types::{extract_types, LimitHit, TypeDefinition, TypeKind};
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::detect_language;

pub fn execute(arguments: &Value) -> Result<CallToolResult> {
    // Backward-compatible input handling:
    // - legacy: `file_path` for single file
    // - current: `path` for file or directory
    let path_str = arguments["path"]
        .as_str()
        .or_else(|| arguments["file_path"].as_str())
        .ok_or_else(|| eyre::eyre!("Missing or invalid 'path' argument"))?;

    let max_tokens = arguments["max_tokens"].as_u64().unwrap_or(2000) as usize;
    let limit = arguments["limit"].as_u64().map(|v| v as usize);
    let offset = arguments["offset"].as_u64().unwrap_or(0) as usize;
    let include_deps = arguments["include_deps"].as_bool().unwrap_or(false);

    let pattern = arguments["pattern"].as_str();

    let path = Path::new(path_str);
    if !path.exists() {
        let response = json!({
            "error": format!("Path does not exist: {path_str}"),
        });
        return Ok(CallToolResult::success(response.to_string()));
    }

    // If `pattern` looks like a glob, treat it as a file filter for extraction.
    // Otherwise treat it as a name filter.
    let (file_glob, name_filter) = match pattern {
        Some(pat) if looks_like_glob(pat) => (Some(pat), None),
        Some(pat) => (None, Some(pat)),
        None => (None, None),
    };

    // 1) Extract types (with 1000 type limit)
    let mut extraction_result = extract_types(path, file_glob, 1000)?;

    // 2) Count usages (directory-wide if path is dir)
    count_all_usages(&mut extraction_result.types, path)?;

    // 3) Sort by usage_count DESC, then name ASC
    extraction_result.types.sort_by(|a, b| {
        b.usage_count
            .cmp(&a.usage_count)
            .then_with(|| a.name.cmp(&b.name))
    });

    // 3.5) Optional name filtering
    let mut filtered: Vec<TypeDefinition> = match name_filter {
        Some(filter) => extraction_result
            .types
            .into_iter()
            .filter(|t| t.name.contains(filter))
            .collect(),
        None => extraction_result.types,
    };

    // 3.6) Pagination (legacy)
    if offset > 0 {
        filtered = filtered.into_iter().skip(offset).collect();
    }
    if let Some(limit) = limit {
        filtered.truncate(limit);
    }

    // 4) Token-aware truncation (new schema uses this; legacy output derives from it)
    let (final_types, limit_hit) = truncate_to_tokens(filtered, max_tokens);

    let language = detect_top_level_language(path_str, path);

    // Legacy grouped output (for existing test suite compatibility)
    let legacy = build_legacy_output(&final_types, language.as_str(), include_deps);

    // New schema output + legacy fields
    let mut response = json!({
        "types": final_types,
        "truncated": limit_hit.is_some() || extraction_result.limit_hit.is_some(),
        "total_types": extraction_result.total_types,
        "types_included": legacy_total_count(&legacy),
        "limit_hit": limit_hit.or(extraction_result.limit_hit),
    });

    if let Some(obj) = response.as_object_mut() {
        if let Some(legacy_obj) = legacy.as_object() {
            for (k, v) in legacy_obj {
                obj.insert(k.clone(), v.clone());
            }
        }
    }

    Ok(CallToolResult::success(response.to_string()))
}

fn detect_top_level_language(path_str: &str, path: &Path) -> String {
    if path.is_dir() {
        return "Mixed".to_string();
    }

    match detect_language(path_str) {
        Ok(lang) => lang.name().to_string(),
        Err(_) => "Unknown".to_string(),
    }
}

fn looks_like_glob(pattern: &str) -> bool {
    pattern.contains('*')
        || pattern.contains('?')
        || pattern.contains('[')
        || pattern.contains('{')
        || pattern.contains('/')
}

fn truncate_to_tokens(
    types: Vec<TypeDefinition>,
    max_tokens: usize,
) -> (Vec<TypeDefinition>, Option<LimitHit>) {
    let bpe = cl100k_base().unwrap();
    let mut result = Vec::new();
    let mut current_tokens = 0;

    for type_def in types {
        let serialized = serde_json::to_string(&type_def).unwrap_or_default();
        let tokens = bpe.encode_with_special_tokens(&serialized).len();

        // Add some overhead for JSON structure (commas, array brackets)
        if current_tokens + tokens + 2 > max_tokens {
            return (result, Some(LimitHit::TokenLimit));
        }

        current_tokens += tokens + 2;
        result.push(type_def);
    }

    (result, None)
}

fn build_legacy_output(types: &[TypeDefinition], language: &str, include_deps: bool) -> Value {
    let mut interfaces = Vec::new();
    let mut classes = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut traits = Vec::new();
    let mut type_aliases = Vec::new();
    let mut others = Vec::new();

    for ty in types {
        let file = path_utils::to_relative_path(ty.file.to_string_lossy().as_ref());

        match ty.kind {
            TypeKind::Interface => {
                let mut fields = Vec::new();
                let mut members = Vec::new();
                if let Some(def_members) = &ty.members {
                    for m in def_members {
                        if m.type_annotation.contains('(') {
                            members.push(json!({
                                "name": m.name,
                                "signature": m.type_annotation,
                                "visibility": "public",
                            }));
                        } else {
                            fields.push(json!({
                                "name": m.name,
                                "type": m.type_annotation,
                                "visibility": "public",
                            }));
                        }
                    }
                }

                interfaces.push(json!({
                    "name": ty.name,
                    "file": file,
                    "line": ty.line,
                    "signature": ty.signature,
                    "usage_count": ty.usage_count,
                    "fields": fields,
                    "members": members,
                    "visibility": "public",
                }));
            }
            TypeKind::Class => {
                classes.push(legacy_class_like(ty, &file));
            }
            TypeKind::Struct => {
                let mut value = legacy_class_like(ty, &file);
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("kind".to_string(), json!("struct"));
                }
                structs.push(value);
            }
            TypeKind::Enum => {
                let variants: Vec<Value> = ty
                    .variants
                    .as_ref()
                    .map(|v| {
                        v.iter()
                            .map(|variant| {
                                json!({
                                    "name": variant.name,
                                    "type": variant.type_annotation,
                                    "visibility": "public",
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                enums.push(json!({
                    "name": ty.name,
                    "file": file,
                    "line": ty.line,
                    "signature": ty.signature,
                    "usage_count": ty.usage_count,
                    "variants": variants,
                    "visibility": "public",
                }));
            }
            TypeKind::Trait | TypeKind::Protocol => {
                let members: Vec<Value> = ty
                    .members
                    .as_ref()
                    .map(|m| {
                        m.iter()
                            .map(|member| {
                                json!({
                                    "name": member.name,
                                    "signature": member.type_annotation,
                                    "visibility": "public",
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                traits.push(json!({
                    "name": ty.name,
                    "file": file,
                    "line": ty.line,
                    "signature": ty.signature,
                    "usage_count": ty.usage_count,
                    "members": members,
                    "visibility": "public",
                }));
            }
            TypeKind::TypeAlias => {
                type_aliases.push(json!({
                    "name": ty.name,
                    "file": file,
                    "line": ty.line,
                    "signature": ty.signature,
                    "usage_count": ty.usage_count,
                    "visibility": "public",
                }));
            }
            TypeKind::Record | TypeKind::TypedDict | TypeKind::NamedTuple => {
                others.push(json!({
                    "name": ty.name,
                    "file": file,
                    "line": ty.line,
                    "signature": ty.signature,
                    "usage_count": ty.usage_count,
                    "visibility": "public",
                }));
            }
        }
    }

    let mut result = json!({
        "language": language,
        "interfaces": interfaces,
        "classes": classes,
        "structs": structs,
        "enums": enums,
        "traits": traits,
        "type_aliases": type_aliases,
        "others": others,
    });

    if include_deps {
        if let Some(obj) = result.as_object_mut() {
            obj.insert("includes_dependencies".to_string(), json!(true));
            obj.insert("dependencies".to_string(), json!([]));
        }
    }

    result
}

fn legacy_class_like(ty: &TypeDefinition, file: &str) -> Value {
    let fields: Vec<Value> = ty
        .fields
        .as_ref()
        .map(|f| {
            f.iter()
                .map(|field| {
                    json!({
                        "name": field.name,
                        "type": field.type_annotation,
                        "visibility": "public",
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let members: Vec<Value> = ty
        .members
        .as_ref()
        .map(|m| {
            m.iter()
                .map(|member| {
                    json!({
                        "name": member.name,
                        "signature": member.type_annotation,
                        "visibility": "public",
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    json!({
        "name": ty.name,
        "file": file,
        "line": ty.line,
        "signature": ty.signature,
        "usage_count": ty.usage_count,
        "fields": fields,
        "members": members,
        "visibility": "public",
    })
}

fn legacy_total_count(legacy: &Value) -> usize {
    let mut total = 0;
    for key in [
        "interfaces",
        "classes",
        "structs",
        "enums",
        "traits",
        "type_aliases",
        "others",
    ] {
        total += legacy
            .get(key)
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0);
    }
    total
}

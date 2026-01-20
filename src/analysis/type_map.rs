use std::path::Path;

use eyre::Result;
use serde_json::{json, Map, Value};
use tiktoken_rs::cl100k_base;

use crate::analysis::path_utils;
use crate::common::budget::BudgetTracker;
use crate::common::{budget, format};
use crate::extraction::types::{extract_types_with_options, TypeDefinition, TypeKind};
use crate::mcp_types::{CallToolResult, CallToolResultExt};

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
    let count_usages = arguments["count_usages"].as_bool().unwrap_or(true);

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

    // 1) Extract types with optional usage counting in a single pass
    let mut extraction_result = extract_types_with_options(path, file_glob, 1000, count_usages)?;

    // 2) Sort by usage_count DESC (if counted), then name ASC
    extraction_result.types.sort_by(|a, b| {
        if count_usages {
            b.usage_count
                .cmp(&a.usage_count)
                .then_with(|| a.name.cmp(&b.name))
        } else {
            // When not counting usages, sort by file path then line number for predictable output
            a.file.cmp(&b.file).then_with(|| a.line.cmp(&b.line))
        }
    });

    // 4) Optional name filtering
    let mut filtered: Vec<TypeDefinition> = match name_filter {
        Some(filter) => extraction_result
            .types
            .into_iter()
            .filter(|t| t.name.contains(filter))
            .collect(),
        None => extraction_result.types,
    };

    // 5) Pagination
    if offset > 0 {
        filtered = filtered.into_iter().skip(offset).collect();
    }
    if let Some(limit) = limit {
        filtered.truncate(limit);
    }

    // 6) Build compact output
    // `BudgetTracker` uses a conservative estimate; final enforcement below uses BPE.
    let mut budget_tracker = BudgetTracker::new((max_tokens * 9) / 10);

    let mut rows = Vec::new();
    let mut truncated = extraction_result.limit_hit.is_some();

    for ty in &filtered {
        let row = type_to_row(ty);
        let estimated = budget::estimate_symbol_tokens(row.len() + 8);
        if !budget_tracker.add(estimated) {
            truncated = true;
            break;
        }
        rows.push(row);
    }

    let mut out = Map::new();
    out.insert("h".to_string(), json!("name|kind|file|line|usage_count"));
    out.insert("types".to_string(), json!(rows.join("\n")));

    // Hard enforcement: drop rows until within token budget.
    let bpe = cl100k_base().unwrap();
    loop {
        let text = serde_json::to_string(&Value::Object(out.clone())).unwrap_or_default();
        if bpe.encode_with_special_tokens(&text).len() <= max_tokens {
            break;
        }

        truncated = true;

        let Some(types_str) = out.get("types").and_then(Value::as_str) else {
            break;
        };

        if types_str.is_empty() {
            break;
        }

        let mut lines: Vec<&str> = types_str.lines().collect();
        lines.pop();
        out.insert("types".to_string(), json!(lines.join("\n")));
    }

    if truncated {
        out.insert("@".to_string(), json!({"t": true}));
    }

    Ok(CallToolResult::success(
        serde_json::to_string(&Value::Object(out)).unwrap_or_default(),
    ))
}

fn type_to_row(ty: &TypeDefinition) -> String {
    let file = path_utils::to_relative_path(ty.file.to_string_lossy().as_ref());
    let kind = type_kind_str(ty.kind);
    let line = ty.line.to_string();
    let usage = ty.usage_count.to_string();

    let owned = [
        ty.name.as_str(),
        kind,
        file.as_str(),
        line.as_str(),
        usage.as_str(),
    ];
    format::format_row(&owned)
}

fn type_kind_str(kind: TypeKind) -> &'static str {
    match kind {
        TypeKind::Interface => "interface",
        TypeKind::Class => "class",
        TypeKind::Struct => "struct",
        TypeKind::Enum => "enum",
        TypeKind::Trait => "trait",
        TypeKind::Protocol => "protocol",
        TypeKind::TypeAlias => "type_alias",
        TypeKind::Record => "record",
        TypeKind::TypedDict => "typed_dict",
        TypeKind::NamedTuple => "named_tuple",
    }
}

fn looks_like_glob(pattern: &str) -> bool {
    pattern.contains('*')
        || pattern.contains('?')
        || pattern.contains('[')
        || pattern.contains('{')
        || pattern.contains('/')
}

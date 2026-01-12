use std::path::PathBuf;

mod common;

use serde_json::json;

#[test]
fn test_find_single_template_struct() {
    // RED: This test will fail because askama module doesn't exist yet
    let fixture_path = PathBuf::from("tests/fixtures/askama_project");
    let template_path = fixture_path.join("templates/calculator.html");

    let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
        &template_path,
        &fixture_path,
    );

    assert!(result.is_ok(), "Should successfully find template structs");
    let structs = result.unwrap();

    // Debug: print what we found
    for s in &structs {
        eprintln!("Found struct: {} in {:?}", s.struct_name, s.file_path);
    }

    #[test]
    fn test_nested_type_resolution_level_1() {
        // RED: Test that nested types are resolved (level 1)
        let fixture_path = PathBuf::from("tests/fixtures/askama_project");
        let template_path = fixture_path.join("templates/admin/dashboard.html");

        let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
            &template_path,
            &fixture_path,
        );

        assert!(result.is_ok());
        let structs = result.unwrap();
        assert_eq!(structs.len(), 1);

        let dashboard = &structs[0];
        assert_eq!(dashboard.struct_name, "DashboardTemplate");

        // Find the stats field
        let stats_field = dashboard
            .fields
            .iter()
            .find(|f| f.name == "stats")
            .expect("Should have stats field");

        assert_eq!(stats_field.field_type, "Statistics");

        // Check nested definition is present
        assert!(
            stats_field.nested_definition.is_some(),
            "Statistics type should be resolved"
        );

        let nested = stats_field.nested_definition.as_ref().unwrap();
        assert_eq!(nested.type_name, "Statistics");
        assert_eq!(nested.depth, 1);

        // Should have: total_users, active_sessions, performance
        assert_eq!(nested.fields.len(), 3);

        let total_users = nested.fields.iter().find(|f| f.name == "total_users");
        assert!(total_users.is_some());
        assert_eq!(total_users.unwrap().field_type, "u32");
    }

    #[test]
    fn test_nested_type_resolution_level_2() {
        // RED: Test that nested types are resolved up to level 2
        let fixture_path = PathBuf::from("tests/fixtures/askama_project");
        let template_path = fixture_path.join("templates/admin/dashboard.html");

        let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
            &template_path,
            &fixture_path,
        );

        assert!(result.is_ok());
        let structs = result.unwrap();
        let dashboard = &structs[0];

        // Navigate: DashboardTemplate -> stats: Statistics -> performance: PerformanceMetrics
        let stats_field = dashboard.fields.iter().find(|f| f.name == "stats").unwrap();

        let stats_nested = stats_field.nested_definition.as_ref().unwrap();

        // Find performance field in Statistics
        let performance_field = stats_nested
            .fields
            .iter()
            .find(|f| f.name == "performance")
            .expect("Statistics should have performance field");

        assert_eq!(performance_field.field_type, "PerformanceMetrics");

        // Check level 2 nested definition
        assert!(
            performance_field.nested_definition.is_some(),
            "PerformanceMetrics should be resolved at level 2"
        );

        let perf_nested = performance_field.nested_definition.as_ref().unwrap();
        assert_eq!(perf_nested.type_name, "PerformanceMetrics");
        assert_eq!(perf_nested.depth, 2);

        // Should have: avg_response_time_ms, error_rate, detailed_stats
        assert_eq!(perf_nested.fields.len(), 3);
    }

    #[test]
    fn test_nested_type_resolution_level_3() {
        // RED: Test that nested types are resolved up to level 3 (max depth)
        let fixture_path = PathBuf::from("tests/fixtures/askama_project");
        let template_path = fixture_path.join("templates/admin/dashboard.html");

        let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
            &template_path,
            &fixture_path,
        );

        assert!(result.is_ok());
        let structs = result.unwrap();
        let dashboard = &structs[0];

        // Navigate: DashboardTemplate -> stats -> performance -> detailed_stats
        let stats = dashboard.fields.iter().find(|f| f.name == "stats").unwrap();
        let stats_nested = stats.nested_definition.as_ref().unwrap();
        let performance = stats_nested
            .fields
            .iter()
            .find(|f| f.name == "performance")
            .unwrap();
        let perf_nested = performance.nested_definition.as_ref().unwrap();
        let detailed = perf_nested
            .fields
            .iter()
            .find(|f| f.name == "detailed_stats")
            .unwrap();

        assert_eq!(detailed.field_type, "DetailedStats");

        // Check level 3 nested definition
        assert!(
            detailed.nested_definition.is_some(),
            "DetailedStats should be resolved at level 3"
        );

        let detailed_nested = detailed.nested_definition.as_ref().unwrap();
        assert_eq!(detailed_nested.type_name, "DetailedStats");
        assert_eq!(detailed_nested.depth, 3);

        // Should have: p95_latency, p99_latency, requests_per_second
        assert_eq!(detailed_nested.fields.len(), 3);
    }

    #[test]
    fn test_nested_type_resolution_max_depth() {
        // RED: Test that we don't resolve beyond depth 3
        let fixture_path = PathBuf::from("tests/fixtures/askama_project");
        let template_path = fixture_path.join("templates/admin/dashboard.html");

        let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
            &template_path,
            &fixture_path,
        );

        assert!(result.is_ok());
        let structs = result.unwrap();
        let dashboard = &structs[0];

        // Navigate to depth 3
        let stats = dashboard.fields.iter().find(|f| f.name == "stats").unwrap();
        let stats_nested = stats.nested_definition.as_ref().unwrap();
        let performance = stats_nested
            .fields
            .iter()
            .find(|f| f.name == "performance")
            .unwrap();
        let perf_nested = performance.nested_definition.as_ref().unwrap();
        let detailed = perf_nested
            .fields
            .iter()
            .find(|f| f.name == "detailed_stats")
            .unwrap();
        let detailed_nested = detailed.nested_definition.as_ref().unwrap();

        // At depth 3, fields should NOT have further nested definitions
        for field in &detailed_nested.fields {
            assert!(
                field.nested_definition.is_none(),
                "Fields at depth 3 should not have nested definitions (max depth reached)"
            );
        }
    }

    // Should find exactly one struct: CalculatorTemplate
    assert_eq!(structs.len(), 1, "Should find exactly one template struct");

    let template_struct = &structs[0];
    assert_eq!(template_struct.struct_name, "CalculatorTemplate");

    // Should have two fields: result and history
    assert_eq!(template_struct.fields.len(), 2, "Should have 2 fields");

    // Check first field: result: i32
    let result_field = template_struct
        .fields
        .iter()
        .find(|f| f.name == "result")
        .expect("Should have 'result' field");
    assert_eq!(result_field.field_type, "i32");
    assert!(
        result_field.nested_definition.is_none(),
        "Primitive type should not have nested definition"
    );

    // Check second field: history: Vec<String>
    let history_field = template_struct
        .fields
        .iter()
        .find(|f| f.name == "history")
        .expect("Should have 'history' field");
    assert_eq!(history_field.field_type, "Vec<String>");
    assert!(
        history_field.nested_definition.is_none(),
        "Vec<String> should not have nested definition"
    );

    // Verify file path points to templates.rs
    assert!(template_struct.file_path.ends_with("src/templates.rs"));

    // Verify line number is reasonable (should be around line 4-6)
    assert!(template_struct.line > 0 && template_struct.line < 20);
}

#[test]
fn test_find_nested_path_template() {
    // RED: Test for nested path like admin/dashboard.html
    let fixture_path = PathBuf::from("tests/fixtures/askama_project");
    let template_path = fixture_path.join("templates/admin/dashboard.html");

    let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
        &template_path,
        &fixture_path,
    );

    assert!(result.is_ok());
    let structs = result.unwrap();

    assert_eq!(structs.len(), 1);
    assert_eq!(structs[0].struct_name, "DashboardTemplate");
}

#[test]
fn test_find_multiple_structs_same_template() {
    // RED: Test finding multiple structs for the same template
    let fixture_path = PathBuf::from("tests/fixtures/askama_project");
    let template_path = fixture_path.join("templates/shared/form.html");

    let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
        &template_path,
        &fixture_path,
    );

    assert!(result.is_ok());
    let structs = result.unwrap();

    // Should find both LoginForm and RegisterForm
    assert_eq!(
        structs.len(),
        2,
        "Should find 2 structs using the same template"
    );

    let struct_names: Vec<&str> = structs.iter().map(|s| s.struct_name.as_str()).collect();
    assert!(struct_names.contains(&"LoginForm"));
    assert!(struct_names.contains(&"RegisterForm"));
}

#[test]
fn test_extract_all_field_types() {
    // RED: Test extraction of various field types
    let fixture_path = PathBuf::from("tests/fixtures/askama_project");
    let template_path = fixture_path.join("templates/shared/form.html");

    let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
        &template_path,
        &fixture_path,
    );

    assert!(result.is_ok());
    let structs = result.unwrap();

    // Find LoginForm
    let login_form = structs
        .iter()
        .find(|s| s.struct_name == "LoginForm")
        .expect("Should find LoginForm");

    // Should have: username: String, error: Option<String>
    assert_eq!(login_form.fields.len(), 2);

    let username_field = login_form
        .fields
        .iter()
        .find(|f| f.name == "username")
        .expect("Should have username field");
    assert_eq!(username_field.field_type, "String");

    let error_field = login_form
        .fields
        .iter()
        .find(|f| f.name == "error")
        .expect("Should have error field");
    assert_eq!(error_field.field_type, "Option<String>");

    // Find RegisterForm
    let register_form = structs
        .iter()
        .find(|s| s.struct_name == "RegisterForm")
        .expect("Should find RegisterForm");

    // Should have: username, email, errors: Vec<String>
    assert_eq!(register_form.fields.len(), 3);

    let errors_field = register_form
        .fields
        .iter()
        .find(|f| f.name == "errors")
        .expect("Should have errors field");
    assert_eq!(errors_field.field_type, "Vec<String>");
}

#[test]
fn test_no_template_structs_found() {
    // RED: Test for template with no matching Rust struct
    let fixture_path = PathBuf::from("tests/fixtures/askama_project");
    // Use a template that doesn't exist in the Rust code
    let template_path = fixture_path.join("templates/nonexistent.html");

    let result = treesitter_mcp::analysis::askama::find_askama_structs_for_template(
        &template_path,
        &fixture_path,
    );

    // Should return Ok with empty vector, not an error
    assert!(result.is_ok());
    let structs = result.unwrap();
    assert_eq!(
        structs.len(),
        0,
        "Should return empty vector for non-existent template"
    );
}

#[test]
fn test_template_context_execute_compact_schema() {
    let fixture_path = PathBuf::from("tests/fixtures/askama_project");
    let template_path = fixture_path.join("templates/calculator.html");

    let args = json!({"template_path": template_path.to_string_lossy()});
    let result = treesitter_mcp::analysis::askama::execute(&args).expect("execute should succeed");

    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).expect("valid json");

    let tpl = output.get("tpl").and_then(|v| v.as_str()).unwrap_or("");
    common::helpers::assert_path_is_relative(tpl);
    assert!(tpl.contains("templates/calculator.html"));

    assert_eq!(
        output.get("h").and_then(|v| v.as_str()),
        Some("struct|field|type")
    );
    let ctx = output.get("ctx").and_then(|v| v.as_str()).unwrap_or("");
    assert!(ctx.contains("CalculatorTemplate"));
    assert!(ctx.contains("result"));
    assert!(ctx.contains("history"));

    assert_eq!(
        output.get("sh").and_then(|v| v.as_str()),
        Some("struct|file|line")
    );
    let s_rows = output.get("s").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(s_rows);
    assert!(
        rows.iter().any(|r| {
            r.get(0).map(|s| s.as_str()) == Some("CalculatorTemplate")
                && r.get(1)
                    .map(|p| p.contains("tests/fixtures/askama_project/src/templates.rs"))
                    .unwrap_or(false)
        }),
        "Should include CalculatorTemplate location"
    );
}

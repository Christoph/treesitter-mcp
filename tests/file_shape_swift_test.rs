//! Unit tests for Swift shape extraction in file_shape.rs
//!
//! Tests verify that extract_shape function properly extracts Swift functions,
//! structs/classes, and imports from Swift source code.

use treesitter_mcp::analysis::file_shape::extract_shape;
use treesitter_mcp::parser::{parse_code, Language};

#[test]
fn test_extract_swift_shape_functions() {
    // Given: Swift code with functions
    let source = r#"
import Foundation

func calculateSum(a: Int, b: Int) -> Int {
    return a + b
}

public func greet(name: String) -> String {
    return "Hello, \(name)!"
}

private func helper() {
    print("Helper")
}
"#;

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift code");
    let shape = extract_shape(&tree, source, Language::Swift).expect("Failed to extract shape");

    // Then: Should extract all functions
    assert_eq!(
        shape.functions.len(),
        3,
        "Should extract 3 functions, got {}",
        shape.functions.len()
    );

    // Verify function names
    let func_names: Vec<&str> = shape.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        func_names.contains(&"calculateSum"),
        "Should contain calculateSum function"
    );
    assert!(
        func_names.contains(&"greet"),
        "Should contain greet function"
    );
    assert!(
        func_names.contains(&"helper"),
        "Should contain helper function"
    );

    // Verify line numbers are positive
    for func in &shape.functions {
        assert!(
            func.line > 0,
            "Function {} should have positive line number",
            func.name
        );
    }
}

#[test]
fn test_extract_swift_shape_structs() {
    // Given: Swift code with structs
    let source = r#"
struct Point {
    var x: Int
    var y: Int
}

public struct Rectangle {
    let width: Double
    let height: Double
    
    func area() -> Double {
        return width * height
    }
}
"#;

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift code");
    let shape = extract_shape(&tree, source, Language::Swift).expect("Failed to extract shape");

    // Then: Should extract all structs
    assert_eq!(
        shape.structs.len(),
        2,
        "Should extract 2 structs, got {}",
        shape.structs.len()
    );

    // Verify struct names
    let struct_names: Vec<&str> = shape.structs.iter().map(|s| s.name.as_str()).collect();
    assert!(
        struct_names.contains(&"Point"),
        "Should contain Point struct"
    );
    assert!(
        struct_names.contains(&"Rectangle"),
        "Should contain Rectangle struct"
    );

    // Verify line numbers
    for s in &shape.structs {
        assert!(
            s.line > 0,
            "Struct {} should have positive line number",
            s.name
        );
    }
}

#[test]
fn test_extract_swift_shape_classes() {
    // Given: Swift code with classes
    let source = r#"
class Vehicle {
    var speed: Int = 0
    
    func accelerate() {
        speed += 10
    }
}

public class Car: Vehicle {
    var brand: String
    
    init(brand: String) {
        self.brand = brand
        super.init()
    }
}
"#;

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift code");
    let shape = extract_shape(&tree, source, Language::Swift).expect("Failed to extract shape");

    // Then: Should extract all classes
    assert_eq!(
        shape.classes.len(),
        2,
        "Should extract 2 classes, got {}",
        shape.classes.len()
    );

    // Verify class names
    let class_names: Vec<&str> = shape.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(
        class_names.contains(&"Vehicle"),
        "Should contain Vehicle class"
    );
    assert!(class_names.contains(&"Car"), "Should contain Car class");

    // Verify line numbers
    for c in &shape.classes {
        assert!(
            c.line > 0,
            "Class {} should have positive line number",
            c.name
        );
    }
}

#[test]
fn test_extract_swift_shape_imports() {
    // Given: Swift code with various imports
    let source = r#"
import Foundation
import UIKit
import SwiftUI

struct ContentView: View {
    var body: some View {
        Text("Hello")
    }
}
"#;

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift code");
    let shape = extract_shape(&tree, source, Language::Swift).expect("Failed to extract shape");

    // Then: Should extract all imports
    assert_eq!(
        shape.imports.len(),
        3,
        "Should extract 3 imports, got {}",
        shape.imports.len()
    );

    // Verify import statements
    assert!(
        shape.imports.contains(&"import Foundation".to_string()),
        "Should contain 'import Foundation'"
    );
    assert!(
        shape.imports.contains(&"import UIKit".to_string()),
        "Should contain 'import UIKit'"
    );
    assert!(
        shape.imports.contains(&"import SwiftUI".to_string()),
        "Should contain 'import SwiftUI'"
    );
}

#[test]
fn test_extract_swift_shape_mixed_declarations() {
    // Given: Swift code with functions, structs, classes, and imports
    let source = r#"
import Foundation

func globalFunction() {
    print("Global")
}

struct DataModel {
    var id: Int
}

class Manager {
    var data: DataModel
    
    init(data: DataModel) {
        self.data = data
    }
}

func anotherFunction() -> String {
    return "test"
}
"#;

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift code");
    let shape = extract_shape(&tree, source, Language::Swift).expect("Failed to extract shape");

    // Then: Should extract all elements
    assert_eq!(shape.functions.len(), 2, "Should have 2 functions");
    assert_eq!(shape.structs.len(), 1, "Should have 1 struct");
    assert_eq!(shape.classes.len(), 1, "Should have 1 class");
    assert_eq!(shape.imports.len(), 1, "Should have 1 import");

    // Verify correct extraction
    assert!(shape.functions.iter().any(|f| f.name == "globalFunction"));
    assert!(shape.functions.iter().any(|f| f.name == "anotherFunction"));
    assert!(shape.structs.iter().any(|s| s.name == "DataModel"));
    assert!(shape.classes.iter().any(|c| c.name == "Manager"));
}

#[test]
fn test_extract_swift_shape_protocols() {
    // Given: Swift code with protocols (which are similar to structs/classes)
    let source = r#"
protocol Drawable {
    func draw()
}

protocol Identifiable {
    var id: String { get }
}
"#;

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift code");
    let shape = extract_shape(&tree, source, Language::Swift).expect("Failed to extract shape");

    // Then: Protocols might be extracted as structs or have a separate category
    // At minimum, we should not crash and should handle them gracefully
    // The implementation can decide whether to include them in structs or classes
    assert!(
        shape.structs.len() > 0 || shape.classes.len() > 0,
        "Should extract protocols as either structs or classes"
    );
}

#[test]
fn test_extract_swift_shape_empty_file() {
    // Given: Empty Swift file
    let source = "";

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Swift).expect("Failed to parse empty Swift code");
    let shape = extract_shape(&tree, source, Language::Swift).expect("Failed to extract shape");

    // Then: Should return empty shape without errors
    assert_eq!(shape.functions.len(), 0);
    assert_eq!(shape.structs.len(), 0);
    assert_eq!(shape.classes.len(), 0);
    assert_eq!(shape.imports.len(), 0);
}

#[test]
fn test_extract_swift_shape_extensions() {
    // Given: Swift code with extensions
    let source = r#"
struct Point {
    var x: Int
    var y: Int
}

extension Point {
    func distance() -> Double {
        return sqrt(Double(x * x + y * y))
    }
}
"#;

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift code");
    let shape = extract_shape(&tree, source, Language::Swift).expect("Failed to extract shape");

    // Then: Should extract the struct
    // Extension methods may or may not be included - that's an implementation detail
    assert_eq!(shape.structs.len(), 1, "Should extract Point struct");
    assert!(shape.structs.iter().any(|s| s.name == "Point"));
}

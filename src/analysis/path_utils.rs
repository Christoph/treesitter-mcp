use std::env;
use std::path::{Path, PathBuf};

/// Convert an absolute path to a relative path from project/git root
pub fn to_relative_path(path: &str) -> String {
    let path_buf = PathBuf::from(path);

    // If already relative, return as-is
    if path_buf.is_relative() {
        return path.to_string();
    }

    // Try to find git root first
    if let Some(git_root) = find_git_root(&path_buf) {
        if let Ok(relative) = path_buf.strip_prefix(&git_root) {
            return relative.to_string_lossy().to_string();
        }
    }

    // Fallback: try to find project root (Cargo.toml, package.json, etc.)
    if let Some(project_root) = find_project_root(&path_buf) {
        if let Ok(relative) = path_buf.strip_prefix(&project_root) {
            return relative.to_string_lossy().to_string();
        }
    }

    // Fallback: use current directory
    if let Ok(cwd) = env::current_dir() {
        if let Ok(relative) = path_buf.strip_prefix(&cwd) {
            return relative.to_string_lossy().to_string();
        }
    }

    // Last resort: if path is a file, try to make it relative from its parent
    if path_buf.is_file() {
        if let Some(parent) = path_buf.parent() {
            if let Some(file_name) = path_buf.file_name() {
                // Try to find a reasonable ancestor (look for common dirs like src, tests, etc.)
                let mut current = parent;
                while let Some(p) = current.parent() {
                    let dir_name = current.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if dir_name == "src" || dir_name == "tests" || dir_name == "lib" {
                        if let Ok(relative) = path_buf.strip_prefix(p) {
                            return relative.to_string_lossy().to_string();
                        }
                    }
                    current = p;
                }

                // If all else fails, return just the filename with parent dir
                if let Some(parent_name) = parent.file_name() {
                    return format!(
                        "{}/{}",
                        parent_name.to_string_lossy(),
                        file_name.to_string_lossy()
                    );
                }
            }
        }
    }

    // Absolute last resort: return original path
    path.to_string()
}

/// Find the git root by looking for .git directory
fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = path;

    // If path is a file, start from its parent
    if current.is_file() {
        current = current.parent()?;
    }

    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }

        current = current.parent()?;
    }
}

/// Find project root by looking for common project markers
///
/// Searches upward from the given path for:
/// - Cargo.toml (Rust)
/// - package.json (JavaScript/TypeScript)
/// - pyproject.toml (Python)
/// - go.mod (Go)
/// - .git (Git repository)
pub fn find_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = path;

    // If path is a file, start from its parent
    if current.is_file() {
        current = current.parent()?;
    }

    loop {
        // Check for common project markers
        if current.join("Cargo.toml").exists()
            || current.join("package.json").exists()
            || current.join("pyproject.toml").exists()
            || current.join("go.mod").exists()
            || current.join(".git").exists()
        {
            return Some(current.to_path_buf());
        }

        current = current.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_relative_path_with_absolute() {
        // Test with a real file in the current project
        let current_file = file!(); // This file's path
        let result = to_relative_path(current_file);

        // Should be relative (no absolute markers at the start)
        assert!(
            !result.starts_with("/Users/")
                && !result.starts_with("/home/")
                && !result.starts_with("C:\\"),
            "Result should be relative, got: {}",
            result
        );

        // Should contain the filename
        assert!(
            result.contains("path_utils.rs"),
            "Should contain filename, got: {}",
            result
        );
    }

    #[test]
    fn test_to_relative_path_already_relative() {
        let path = "src/main.rs";
        let result = to_relative_path(path);

        // Should return as-is or similar
        assert!(result.contains("src") || result.contains("main.rs"));
    }
}

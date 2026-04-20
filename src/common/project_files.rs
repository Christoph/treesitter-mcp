use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::{DirEntry, WalkDir};

const LEGACY_IGNORED_DIR_NAMES: &[&str] = &["target", "node_modules", "vendor", "build", "dist"];

#[derive(Debug, Default)]
struct IgnoredPaths {
    exact: HashSet<PathBuf>,
    prefixes: Vec<PathBuf>,
}

impl IgnoredPaths {
    fn for_root(root: &Path) -> Self {
        let current_dir = if root.is_dir() {
            root
        } else {
            root.parent().unwrap_or(root)
        };

        let Ok(repo_root) = git_repo_root(current_dir) else {
            return Self::default();
        };

        let mut args = vec![
            "ls-files",
            "--others",
            "-i",
            "--exclude-standard",
            "--directory",
            "--",
        ];

        let scoped_root = root
            .canonicalize()
            .unwrap_or_else(|_| root.to_path_buf())
            .strip_prefix(&repo_root)
            .ok()
            .map(Path::to_path_buf);

        if let Some(rel_root) = scoped_root
            .as_ref()
            .filter(|path| !path.as_os_str().is_empty())
        {
            if let Some(rel_root_str) = rel_root.to_str() {
                args.push(rel_root_str);
            } else {
                return Self::default();
            }
        }

        let output = match Command::new("git")
            .args(&args)
            .current_dir(&repo_root)
            .output()
        {
            Ok(output) if output.status.success() => output,
            _ => return Self::default(),
        };

        let mut ignored = Self::default();
        for raw_line in String::from_utf8_lossy(&output.stdout).lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let is_dir = trimmed.ends_with('/');
            let relative = trimmed.trim_end_matches('/');
            let absolute = repo_root.join(relative);
            if is_dir {
                ignored.prefixes.push(absolute);
            } else {
                ignored.exact.insert(absolute);
            }
        }

        ignored
    }

    fn contains(&self, path: &Path) -> bool {
        let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        self.exact.contains(&normalized)
            || self
                .prefixes
                .iter()
                .any(|prefix| normalized == *prefix || normalized.starts_with(prefix))
    }
}

pub fn collect_project_files(root: &Path) -> Result<Vec<PathBuf>, io::Error> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let ignored = IgnoredPaths::for_root(root);
    let mut files = Vec::new();

    let walker = WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_descend(entry, &ignored));

    for entry in walker {
        let entry = entry
            .map_err(|err| io::Error::other(format!("Failed to walk {}: {err}", root.display())))?;

        if !entry.file_type().is_file() {
            continue;
        }

        if ignored.contains(entry.path()) || is_hidden_name(entry.file_name()) {
            continue;
        }

        files.push(entry.into_path());
    }

    files.sort();
    Ok(files)
}

fn should_descend(entry: &DirEntry, ignored: &IgnoredPaths) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    if ignored.contains(entry.path()) || is_hidden_name(entry.file_name()) {
        return false;
    }

    if !entry.file_type().is_dir() {
        return true;
    }

    !LEGACY_IGNORED_DIR_NAMES.iter().any(|name| {
        entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
    })
}

fn is_hidden_name(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .map(|value| value.starts_with('.'))
        .unwrap_or(false)
}

fn git_repo_root(current_dir: &Path) -> Result<PathBuf, io::Error> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(current_dir)
        .output()
        .map_err(|err| io::Error::other(format!("Failed to locate git root: {err}")))?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Path is not inside a git repository",
        ));
    }

    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

use std::collections::HashSet;
use std::path::Path;

use crate::error::GitError;
use crate::repo::open_repo;

/// Status of a file in the working tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Clean,
    Modified,
    Staged,
    Untracked,
    Deleted,
    Renamed,
    Conflicted,
    Ignored,
}

/// A file with its git status
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileStatusEntry {
    /// Relative path from repo root
    pub path: String,
    /// Git status
    pub status: FileStatus,
}

/// Get the status of all changed files in the repository.
/// Returns only files that differ from HEAD or are untracked.
pub fn get_status(path: &Path) -> Result<Vec<FileStatusEntry>, GitError> {
    let repo = open_repo(path)?;
    let mut entries = Vec::new();

    let index = repo
        .index_or_empty()
        .map_err(|e| GitError::Gix(e.to_string()))?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Bare repository".to_string()))?;

    // Collect indexed paths
    let mut indexed_paths: HashSet<String> = HashSet::new();
    for entry in index.entries() {
        let entry_path = entry.path(&index);
        indexed_paths.insert(entry_path.to_string());
    }

    // Walk working directory and compare with index
    walk_directory(workdir, workdir, &indexed_paths, &mut entries)?;

    // Check for deleted files (in index but not on disk)
    for indexed_path in &indexed_paths {
        let full_path = workdir.join(indexed_path);
        if !full_path.exists() {
            entries.push(FileStatusEntry {
                path: indexed_path.clone(),
                status: FileStatus::Deleted,
            });
        }
    }

    Ok(entries)
}

fn walk_directory(
    base: &Path,
    dir: &Path,
    indexed_paths: &HashSet<String>,
    entries: &mut Vec<FileStatusEntry>,
) -> Result<(), GitError> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| GitError::Path(format!("Cannot read directory {}: {}", dir.display(), e)))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| GitError::Path(e.to_string()))?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip .git directory
        if file_name == ".git" {
            continue;
        }

        // Skip common non-essential directories
        if path.is_dir()
            && (file_name == "node_modules"
                || file_name == "target"
                || file_name == ".next"
                || file_name == "dist")
        {
            continue;
        }

        if path.is_dir() {
            walk_directory(base, &path, indexed_paths, entries)?;
        } else {
            let relative = path
                .strip_prefix(base)
                .map_err(|e| GitError::Path(e.to_string()))?;
            let rel_str = relative.to_string_lossy().replace('\\', "/");

            if !indexed_paths.contains(&rel_str) {
                entries.push(FileStatusEntry {
                    path: rel_str,
                    status: FileStatus::Untracked,
                });
            }
        }
    }

    Ok(())
}

/// Get status for a single file
pub fn get_file_status(repo_path: &Path, file_path: &str) -> Result<FileStatus, GitError> {
    let statuses = get_status(repo_path)?;
    Ok(statuses
        .iter()
        .find(|e| e.path == file_path)
        .map(|e| e.status)
        .unwrap_or(FileStatus::Clean))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn git_cmd(dir: &Path, args: &[&str]) {
        std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git command failed");
    }

    #[test]
    fn test_status_clean() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        git_cmd(tmp.path(), &["add", "file.txt"]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        let statuses = get_status(tmp.path()).unwrap();
        assert!(statuses.is_empty(), "Expected no changed files, got: {:?}", statuses);
    }

    #[test]
    fn test_status_untracked() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("tracked.txt"), "hello").unwrap();
        git_cmd(tmp.path(), &["add", "tracked.txt"]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        // Create untracked file
        std::fs::write(tmp.path().join("new.txt"), "world").unwrap();

        let statuses = get_status(tmp.path()).unwrap();
        let untracked: Vec<_> = statuses
            .iter()
            .filter(|e| e.status == FileStatus::Untracked)
            .collect();
        assert!(
            untracked.iter().any(|e| e.path == "new.txt"),
            "Expected new.txt as untracked, got: {:?}",
            statuses
        );
    }

    #[test]
    fn test_status_deleted() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        git_cmd(tmp.path(), &["add", "file.txt"]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        // Delete the file
        std::fs::remove_file(tmp.path().join("file.txt")).unwrap();

        let statuses = get_status(tmp.path()).unwrap();
        let deleted: Vec<_> = statuses
            .iter()
            .filter(|e| e.status == FileStatus::Deleted)
            .collect();
        assert!(
            deleted.iter().any(|e| e.path == "file.txt"),
            "Expected file.txt as deleted, got: {:?}",
            statuses
        );
    }
}

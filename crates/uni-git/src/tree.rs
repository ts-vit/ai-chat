use std::collections::HashMap;
use std::path::Path;

use crate::error::GitError;
use crate::status::{get_status, FileStatus};

/// A node in the file tree
#[derive(Debug, Clone, serde::Serialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub status: Option<FileStatus>,
    pub children: Vec<TreeNode>,
}

/// Build a file tree for the repository at `path`.
/// Includes git status for each file.
/// `max_depth` limits directory traversal (0 = root only, None = unlimited).
pub fn file_tree(repo_path: &Path, max_depth: Option<usize>) -> Result<Vec<TreeNode>, GitError> {
    let repo = crate::repo::open_repo(repo_path)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Bare repository".to_string()))?;

    let statuses = get_status(repo_path)?;
    let status_map: HashMap<String, FileStatus> =
        statuses.into_iter().map(|e| (e.path, e.status)).collect();

    let depth = max_depth.unwrap_or(usize::MAX);
    build_tree(workdir, workdir, &status_map, 0, depth)
}

fn build_tree(
    base: &Path,
    dir: &Path,
    status_map: &HashMap<String, FileStatus>,
    current_depth: usize,
    max_depth: usize,
) -> Result<Vec<TreeNode>, GitError> {
    if current_depth > max_depth {
        return Ok(vec![]);
    }

    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| GitError::Path(format!("Cannot read {}: {}", dir.display(), e)))?;

    let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        let a_dir = a.path().is_dir();
        let b_dir = b.path().is_dir();
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    let mut nodes = Vec::new();

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }

        let relative = path
            .strip_prefix(base)
            .map_err(|e| GitError::Path(e.to_string()))?;
        let rel_str = relative.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            let children =
                build_tree(base, &path, status_map, current_depth + 1, max_depth)?;
            nodes.push(TreeNode {
                name,
                path: rel_str,
                is_dir: true,
                status: None,
                children,
            });
        } else {
            let status = status_map.get(&rel_str).copied();
            nodes.push(TreeNode {
                name,
                path: rel_str,
                is_dir: false,
                status,
                children: vec![],
            });
        }
    }

    Ok(nodes)
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
    fn test_file_tree_basic() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("README.md"), "# Hello").unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        let tree = file_tree(tmp.path(), None).unwrap();
        assert!(!tree.is_empty());

        // src dir should be first (dirs before files)
        let dir_node = tree.iter().find(|n| n.name == "src");
        assert!(dir_node.is_some());
        assert!(dir_node.unwrap().is_dir);
    }

    #[test]
    fn test_file_tree_skips_dotgit() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "data").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        let tree = file_tree(tmp.path(), None).unwrap();
        assert!(!tree.iter().any(|n| n.name == ".git"));
    }

    #[test]
    fn test_file_tree_max_depth() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        std::fs::write(tmp.path().join("a/b/c/deep.txt"), "deep").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        let tree = file_tree(tmp.path(), Some(1)).unwrap();
        let a = tree.iter().find(|n| n.name == "a").unwrap();
        let b = a.children.iter().find(|n| n.name == "b").unwrap();
        // At depth 1, b's children should be empty
        assert!(b.children.is_empty());
    }

    #[test]
    fn test_file_tree_sorted() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::create_dir(tmp.path().join("zdir")).unwrap();
        std::fs::write(tmp.path().join("afile.txt"), "a").unwrap();
        std::fs::write(tmp.path().join("zdir/inner.txt"), "z").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        let tree = file_tree(tmp.path(), None).unwrap();
        // Directory should come before file
        assert!(tree[0].is_dir, "First entry should be a directory");
    }
}

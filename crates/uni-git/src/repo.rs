use std::path::{Path, PathBuf};

use crate::error::GitError;

/// Information about a discovered git repository
#[derive(Debug, Clone, serde::Serialize)]
pub struct RepoInfo {
    /// Path to the working directory (project root)
    pub workdir: String,
    /// Path to the .git directory
    pub git_dir: String,
    /// Whether the repo is bare
    pub is_bare: bool,
}

/// Open a git repository at the given path.
/// Searches upward from `path` to find the repo root.
pub fn open_repo(path: &Path) -> Result<gix::Repository, GitError> {
    gix::discover(path).map_err(|e| {
        let msg = e.to_string();
        let lower = msg.to_lowercase();
        if lower.contains("not a git repository")
            || lower.contains("could not find")
            || lower.contains("directory")
        {
            GitError::NotARepo(path.display().to_string())
        } else {
            GitError::Gix(msg)
        }
    })
}

/// Get basic info about the repository at `path`.
pub fn repo_info(path: &Path) -> Result<RepoInfo, GitError> {
    let repo = open_repo(path)?;

    let workdir = repo
        .workdir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let git_dir = repo.git_dir().display().to_string();
    let is_bare = repo.is_bare();

    Ok(RepoInfo {
        workdir,
        git_dir,
        is_bare,
    })
}

/// Find the repository root (working directory) from any path inside it.
pub fn find_repo_root(path: &Path) -> Result<PathBuf, GitError> {
    let repo = open_repo(path)?;
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| GitError::Other("Bare repository has no working directory".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn git_init(dir: &Path) {
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git init failed");
    }

    #[test]
    fn test_open_repo() {
        let tmp = TempDir::new().unwrap();
        git_init(tmp.path());
        let repo = open_repo(tmp.path());
        assert!(repo.is_ok());
    }

    #[test]
    fn test_open_repo_not_git() {
        let tmp = TempDir::new().unwrap();
        let result = open_repo(tmp.path());
        assert!(result.is_err());
        assert!(result.is_err(), "Expected error for non-git dir");
    }

    #[test]
    fn test_repo_info() {
        let tmp = TempDir::new().unwrap();
        git_init(tmp.path());
        let info = repo_info(tmp.path()).unwrap();
        assert!(!info.is_bare);
        assert!(!info.workdir.is_empty());
    }

    #[test]
    fn test_find_repo_root() {
        let tmp = TempDir::new().unwrap();
        git_init(tmp.path());
        let subdir = tmp.path().join("sub").join("deep");
        std::fs::create_dir_all(&subdir).unwrap();
        let root = find_repo_root(&subdir).unwrap();
        assert_eq!(
            root.canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap()
        );
    }
}

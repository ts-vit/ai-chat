use std::path::Path;

use crate::error::GitError;
use crate::repo::open_repo;

/// Information about a commit
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
}

/// Information about the current branch
#[derive(Debug, Clone, serde::Serialize)]
pub struct BranchInfo {
    pub name: Option<String>,
    pub head_hash: String,
    pub is_detached: bool,
}

/// Get the current branch info
pub fn current_branch(repo_path: &Path) -> Result<BranchInfo, GitError> {
    let repo = open_repo(repo_path)?;

    let head = repo.head().map_err(|e| GitError::Gix(e.to_string()))?;

    let head_hash = head
        .id()
        .map(|id| id.to_string())
        .unwrap_or_default();

    let is_detached = head.is_detached();

    let name = if is_detached {
        None
    } else {
        head.referent_name().map(|r| {
            let full = r.as_bstr().to_string();
            full.strip_prefix("refs/heads/")
                .unwrap_or(&full)
                .to_string()
        })
    };

    Ok(BranchInfo {
        name,
        head_hash,
        is_detached,
    })
}

/// Get the last N commits from HEAD
pub fn log(repo_path: &Path, max_count: usize) -> Result<Vec<CommitInfo>, GitError> {
    let repo = open_repo(repo_path)?;

    let head_commit = repo
        .head_commit()
        .map_err(|e| GitError::Gix(e.to_string()))?;

    let mut commits = Vec::new();
    let mut current = Some(head_commit);

    while let Some(commit) = current {
        if commits.len() >= max_count {
            break;
        }

        let hash = commit.id().to_string();
        let short_hash = hash[..8.min(hash.len())].to_string();

        let message = commit
            .message()
            .map(|m| m.title.to_string().trim_end().to_string())
            .unwrap_or_default();

        let author = commit
            .author()
            .map(|a| (a.name.to_string(), a.email.to_string()))
            .unwrap_or_else(|_| (String::new(), String::new()));

        let timestamp = commit
            .time()
            .map(|t| t.seconds)
            .unwrap_or(0);

        commits.push(CommitInfo {
            hash,
            short_hash,
            message,
            author_name: author.0,
            author_email: author.1,
            timestamp,
        });

        current = commit
            .parent_ids()
            .next()
            .and_then(|parent_id| parent_id.object().ok())
            .and_then(|obj| obj.try_into_commit().ok());
    }

    Ok(commits)
}

/// List all local branch names
pub fn list_branches(repo_path: &Path) -> Result<Vec<String>, GitError> {
    let repo = open_repo(repo_path)?;

    let refs = repo
        .references()
        .map_err(|e| GitError::Gix(e.to_string()))?;

    let branches: Vec<String> = refs
        .local_branches()
        .map_err(|e| GitError::Gix(e.to_string()))?
        .filter_map(|r| {
            r.ok().map(|reference| {
                let name = reference.name().as_bstr().to_string();
                name.strip_prefix("refs/heads/")
                    .unwrap_or(&name)
                    .to_string()
            })
        })
        .collect();

    Ok(branches)
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
    fn test_current_branch() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init", "-b", "main"]);
        std::fs::write(tmp.path().join("file.txt"), "data").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        let branch = current_branch(tmp.path()).unwrap();
        assert_eq!(branch.name, Some("main".to_string()));
        assert!(!branch.is_detached);
        assert!(!branch.head_hash.is_empty());
    }

    #[test]
    fn test_log_single_commit() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "data").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "initial commit"]);

        let commits = log(tmp.path(), 10).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "initial commit");
        assert_eq!(commits[0].author_name, "Test");
        assert_eq!(commits[0].author_email, "test@test.com");
        assert_eq!(commits[0].short_hash.len(), 8);
    }

    #[test]
    fn test_log_multiple_commits() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "v1").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "first"]);
        std::fs::write(tmp.path().join("file.txt"), "v2").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "second"]);
        std::fs::write(tmp.path().join("file.txt"), "v3").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "third"]);

        // Test max_count
        let commits = log(tmp.path(), 2).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "third"); // newest first
        assert_eq!(commits[1].message, "second");
    }

    #[test]
    fn test_list_branches() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init", "-b", "main"]);
        std::fs::write(tmp.path().join("file.txt"), "data").unwrap();
        git_cmd(tmp.path(), &["add", "."]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);
        git_cmd(tmp.path(), &["branch", "feature"]);
        git_cmd(tmp.path(), &["branch", "develop"]);

        let branches = list_branches(tmp.path()).unwrap();
        assert!(branches.contains(&"main".to_string()));
        assert!(branches.contains(&"feature".to_string()));
        assert!(branches.contains(&"develop".to_string()));
    }
}

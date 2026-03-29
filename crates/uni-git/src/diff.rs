use std::path::Path;

use crate::error::GitError;
use crate::repo::open_repo;

/// A line in a diff
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    Header,
}

/// A diff hunk (section of changes)
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// Diff for a single file
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileDiff {
    pub path: String,
    pub hunks: Vec<DiffHunk>,
    pub is_binary: bool,
    pub additions: u32,
    pub deletions: u32,
}

/// Get diff for a specific file between working tree and HEAD.
pub fn diff_file(repo_path: &Path, file_path: &str) -> Result<FileDiff, GitError> {
    let repo = open_repo(repo_path)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Bare repository".to_string()))?;

    let current_path = workdir.join(file_path);
    let current_content = std::fs::read_to_string(&current_path)
        .map_err(|e| GitError::Path(format!("Cannot read {}: {}", current_path.display(), e)))?;

    let old_content = read_file_from_head(&repo, file_path)?;

    let hunks = generate_diff(&old_content.unwrap_or_default(), &current_content);

    let mut additions = 0u32;
    let mut deletions = 0u32;
    for hunk in &hunks {
        for line in &hunk.lines {
            match line.kind {
                DiffLineKind::Addition => additions += 1,
                DiffLineKind::Deletion => deletions += 1,
                _ => {}
            }
        }
    }

    Ok(FileDiff {
        path: file_path.to_string(),
        hunks,
        is_binary: false,
        additions,
        deletions,
    })
}

/// Read a file's content from HEAD commit
fn read_file_from_head(repo: &gix::Repository, file_path: &str) -> Result<Option<String>, GitError> {
    let head = match repo.head_commit() {
        Ok(commit) => commit,
        Err(_) => return Ok(None),
    };

    let tree = head.tree().map_err(|e| GitError::Gix(e.to_string()))?;

    let entry = tree
        .lookup_entry_by_path(file_path)
        .map_err(|e| GitError::Gix(e.to_string()))?;

    match entry {
        Some(entry) => {
            let object = entry.object().map_err(|e| GitError::Gix(e.to_string()))?;
            let data = &object.data;
            match std::str::from_utf8(data) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Ok(None),
            }
        }
        None => Ok(None),
    }
}

/// Simple line-by-line diff using LCS algorithm.
/// Produces unified diff hunks with context lines.
pub(crate) fn generate_diff(old: &str, new: &str) -> Vec<DiffHunk> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    if old_lines == new_lines {
        return vec![];
    }

    let changes = compute_changes(&old_lines, &new_lines);
    build_hunks(&changes, &old_lines, &new_lines, 3)
}

#[derive(Debug, Clone, Copy)]
enum Change {
    Equal(usize, usize),
    Delete(usize),
    Insert(usize),
}

fn compute_changes(old: &[&str], new: &[&str]) -> Vec<Change> {
    let n = old.len();
    let m = new.len();

    // LCS table
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if old[i - 1] == new[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack
    let mut changes = Vec::new();
    let mut i = n;
    let mut j = m;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            changes.push(Change::Equal(i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            changes.push(Change::Insert(j - 1));
            j -= 1;
        } else {
            changes.push(Change::Delete(i - 1));
            i -= 1;
        }
    }

    changes.reverse();
    changes
}

fn build_hunks(
    changes: &[Change],
    old: &[&str],
    new: &[&str],
    context: usize,
) -> Vec<DiffHunk> {
    if changes.is_empty() {
        return vec![];
    }

    // Find ranges of non-Equal changes, then expand with context
    let mut change_indices: Vec<usize> = Vec::new();
    for (idx, change) in changes.iter().enumerate() {
        if !matches!(change, Change::Equal(_, _)) {
            change_indices.push(idx);
        }
    }

    if change_indices.is_empty() {
        return vec![];
    }

    // Group nearby changes into hunks
    let mut groups: Vec<(usize, usize)> = Vec::new(); // (start_idx, end_idx) in changes array
    let mut group_start = change_indices[0];
    let mut group_end = change_indices[0];

    for &ci in &change_indices[1..] {
        // If this change is within 2*context of the previous, merge into same group
        if ci <= group_end + 2 * context + 1 {
            group_end = ci;
        } else {
            groups.push((group_start, group_end));
            group_start = ci;
            group_end = ci;
        }
    }
    groups.push((group_start, group_end));

    let mut hunks = Vec::new();

    for (gs, ge) in groups {
        let hunk_start = if gs >= context { gs - context } else { 0 };
        let hunk_end = (ge + context + 1).min(changes.len());

        let mut lines = Vec::new();
        let mut old_start = None;
        let mut new_start = None;
        let mut old_count = 0u32;
        let mut new_count = 0u32;

        for change in &changes[hunk_start..hunk_end] {
            match change {
                Change::Equal(oi, ni) => {
                    if old_start.is_none() {
                        old_start = Some(*oi as u32 + 1);
                    }
                    if new_start.is_none() {
                        new_start = Some(*ni as u32 + 1);
                    }
                    lines.push(DiffLine {
                        kind: DiffLineKind::Context,
                        content: old[*oi].to_string(),
                        old_line: Some(*oi as u32 + 1),
                        new_line: Some(*ni as u32 + 1),
                    });
                    old_count += 1;
                    new_count += 1;
                }
                Change::Delete(oi) => {
                    if old_start.is_none() {
                        old_start = Some(*oi as u32 + 1);
                    }
                    lines.push(DiffLine {
                        kind: DiffLineKind::Deletion,
                        content: old[*oi].to_string(),
                        old_line: Some(*oi as u32 + 1),
                        new_line: None,
                    });
                    old_count += 1;
                }
                Change::Insert(ni) => {
                    if new_start.is_none() {
                        new_start = Some(*ni as u32 + 1);
                    }
                    lines.push(DiffLine {
                        kind: DiffLineKind::Addition,
                        content: new[*ni].to_string(),
                        old_line: None,
                        new_line: Some(*ni as u32 + 1),
                    });
                    new_count += 1;
                }
            }
        }

        let header = format!(
            "@@ -{},{} +{},{} @@",
            old_start.unwrap_or(1),
            old_count,
            new_start.unwrap_or(1),
            new_count,
        );

        hunks.push(DiffHunk { header, lines });
    }

    hunks
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
    fn test_generate_diff_equal() {
        let hunks = generate_diff("line1\nline2\nline3", "line1\nline2\nline3");
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_generate_diff_simple() {
        let hunks = generate_diff("line1\nline2\nline3", "line1\nmodified\nline3");
        assert!(!hunks.is_empty());
        let hunk = &hunks[0];
        assert!(hunk.lines.iter().any(|l| l.kind == DiffLineKind::Addition));
        assert!(hunk.lines.iter().any(|l| l.kind == DiffLineKind::Deletion));
    }

    #[test]
    fn test_generate_diff_addition_only() {
        let hunks = generate_diff("line1\nline2", "line1\nline2\nline3");
        assert!(!hunks.is_empty());
        let additions: Vec<_> = hunks[0]
            .lines
            .iter()
            .filter(|l| l.kind == DiffLineKind::Addition)
            .collect();
        assert_eq!(additions.len(), 1);
        assert_eq!(additions[0].content, "line3");
    }

    #[test]
    fn test_generate_diff_deletion_only() {
        let hunks = generate_diff("line1\nline2\nline3", "line1\nline2");
        assert!(!hunks.is_empty());
        let deletions: Vec<_> = hunks[0]
            .lines
            .iter()
            .filter(|l| l.kind == DiffLineKind::Deletion)
            .collect();
        assert_eq!(deletions.len(), 1);
        assert_eq!(deletions[0].content, "line3");
    }

    #[test]
    fn test_diff_no_changes() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "hello\nworld\n").unwrap();
        git_cmd(tmp.path(), &["add", "file.txt"]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        let diff = diff_file(tmp.path(), "file.txt").unwrap();
        assert!(diff.hunks.is_empty());
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 0);
    }

    #[test]
    fn test_diff_addition() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "line1\n").unwrap();
        git_cmd(tmp.path(), &["add", "file.txt"]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        std::fs::write(tmp.path().join("file.txt"), "line1\nline2\n").unwrap();

        let diff = diff_file(tmp.path(), "file.txt").unwrap();
        assert!(diff.additions > 0);
    }

    #[test]
    fn test_diff_deletion() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "line1\nline2\nline3\n").unwrap();
        git_cmd(tmp.path(), &["add", "file.txt"]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        std::fs::write(tmp.path().join("file.txt"), "line1\n").unwrap();

        let diff = diff_file(tmp.path(), "file.txt").unwrap();
        assert!(diff.deletions > 0);
    }

    #[test]
    fn test_diff_modification() {
        let tmp = TempDir::new().unwrap();
        git_cmd(tmp.path(), &["init"]);
        std::fs::write(tmp.path().join("file.txt"), "line1\noriginal\nline3\n").unwrap();
        git_cmd(tmp.path(), &["add", "file.txt"]);
        git_cmd(tmp.path(), &["commit", "-m", "init"]);

        std::fs::write(tmp.path().join("file.txt"), "line1\nmodified\nline3\n").unwrap();

        let diff = diff_file(tmp.path(), "file.txt").unwrap();
        assert!(diff.additions > 0);
        assert!(diff.deletions > 0);
    }
}

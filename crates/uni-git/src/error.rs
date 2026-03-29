use std::fmt;

#[derive(Debug)]
pub enum GitError {
    /// Not a git repository
    NotARepo(String),
    /// gix error wrapper
    Gix(String),
    /// Path error
    Path(String),
    /// Generic error
    Other(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotARepo(path) => write!(f, "Not a git repository: {}", path),
            GitError::Gix(msg) => write!(f, "Git error: {}", msg),
            GitError::Path(msg) => write!(f, "Path error: {}", msg),
            GitError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for GitError {}

impl From<GitError> for String {
    fn from(e: GitError) -> Self {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            GitError::NotARepo("/tmp/foo".into()).to_string(),
            "Not a git repository: /tmp/foo"
        );
        assert_eq!(
            GitError::Gix("something broke".into()).to_string(),
            "Git error: something broke"
        );
        assert_eq!(
            GitError::Path("bad path".into()).to_string(),
            "Path error: bad path"
        );
        assert_eq!(
            GitError::Other("misc".into()).to_string(),
            "misc"
        );
    }

    #[test]
    fn test_error_to_string() {
        let e = GitError::NotARepo("/tmp".into());
        let s: String = e.into();
        assert!(s.contains("Not a git repository"));
    }
}

use std::path::{Path, PathBuf};
use uni_common::generate_id;

/// A sandbox for script execution — temp directory with input/output areas.
pub struct Sandbox {
    pub dir: PathBuf,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl Sandbox {
    /// Create a new sandbox in the given base directory.
    pub async fn create(base_dir: &Path) -> Result<Self, uni_common::UniError> {
        let id = generate_id();
        let dir = base_dir.join("sandboxes").join(&id);
        let input_dir = dir.join("input");
        let output_dir = dir.join("output");

        tokio::fs::create_dir_all(&input_dir)
            .await
            .map_err(uni_common::UniError::Io)?;
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(uni_common::UniError::Io)?;

        Ok(Self {
            dir,
            input_dir,
            output_dir,
        })
    }

    /// Copy a file into the sandbox input area.
    /// Returns the path inside the sandbox.
    pub async fn add_input_file(&self, source: &Path) -> Result<PathBuf, uni_common::UniError> {
        let file_name = source
            .file_name()
            .ok_or_else(|| uni_common::UniError::Generic("No filename".to_string()))?;
        let dest = self.input_dir.join(file_name);
        tokio::fs::copy(source, &dest)
            .await
            .map_err(uni_common::UniError::Io)?;
        Ok(dest)
    }

    /// List files in the output directory (produced by the script).
    pub async fn output_files(&self) -> Result<Vec<PathBuf>, uni_common::UniError> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.output_dir)
            .await
            .map_err(uni_common::UniError::Io)?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(uni_common::UniError::Io)?
        {
            if entry
                .file_type()
                .await
                .map_err(uni_common::UniError::Io)?
                .is_file()
            {
                files.push(entry.path());
            }
        }
        Ok(files)
    }

    /// Clean up the sandbox directory.
    pub async fn cleanup(self) -> Result<(), uni_common::UniError> {
        if self.dir.exists() {
            tokio::fs::remove_dir_all(&self.dir)
                .await
                .map_err(uni_common::UniError::Io)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_create_and_cleanup() {
        let base = std::env::temp_dir().join("uni-python-sandbox-test");
        let sandbox = Sandbox::create(&base).await.unwrap();
        assert!(sandbox.dir.exists());
        assert!(sandbox.input_dir.exists());
        assert!(sandbox.output_dir.exists());

        let dir = sandbox.dir.clone();
        sandbox.cleanup().await.unwrap();
        assert!(!dir.exists());
        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_sandbox_add_input_file() {
        let base = std::env::temp_dir().join("uni-python-sandbox-input-test");
        let sandbox = Sandbox::create(&base).await.unwrap();

        // Create a temp file to copy in
        let src = std::env::temp_dir().join("uni-python-test-input.txt");
        tokio::fs::write(&src, "test content").await.unwrap();

        let dest = sandbox.add_input_file(&src).await.unwrap();
        assert!(dest.exists());
        let content = tokio::fs::read_to_string(&dest).await.unwrap();
        assert_eq!(content, "test content");

        sandbox.cleanup().await.unwrap();
        let _ = tokio::fs::remove_file(&src).await;
        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_sandbox_output_files() {
        let base = std::env::temp_dir().join("uni-python-sandbox-output-test");
        let sandbox = Sandbox::create(&base).await.unwrap();

        // Simulate script writing output
        tokio::fs::write(sandbox.output_dir.join("result.md"), "# Hello")
            .await
            .unwrap();
        tokio::fs::write(sandbox.output_dir.join("meta.json"), "{}")
            .await
            .unwrap();

        let files = sandbox.output_files().await.unwrap();
        assert_eq!(files.len(), 2);

        sandbox.cleanup().await.unwrap();
        let _ = tokio::fs::remove_dir_all(&base).await;
    }
}

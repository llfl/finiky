use super::{FileSystem, FileSystemError};
use std::path::{Path, PathBuf};

pub struct DirectoryFileSystem {
    root: PathBuf,
}

impl DirectoryFileSystem {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, FileSystemError> {
        let root = root.as_ref().canonicalize()?;
        if !root.is_dir() {
            return Err(FileSystemError::InvalidPath(
                "Path is not a directory".to_string(),
            ));
        }
        Ok(DirectoryFileSystem { root })
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, FileSystemError> {
        // Normalize path by removing leading slashes and resolving ".."
        let normalized = path.trim_start_matches('/');
        let path_buf = PathBuf::from(normalized);

        // Resolve to absolute path
        let full_path = self
            .root
            .join(&path_buf)
            .canonicalize()
            .map_err(|_| FileSystemError::NotFound(path.to_string()))?;

        // Ensure the resolved path is within the root directory (prevent directory traversal)
        if !full_path.starts_with(&self.root) {
            return Err(FileSystemError::InvalidPath(
                "Path traversal detected".to_string(),
            ));
        }

        Ok(full_path)
    }
}

#[async_trait::async_trait]
impl FileSystem for DirectoryFileSystem {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, FileSystemError> {
        let file_path = self.resolve_path(path)?;

        if !file_path.is_file() {
            return Err(FileSystemError::NotFound(path.to_string()));
        }

        tokio::fs::read(&file_path)
            .await
            .map_err(FileSystemError::Io)
    }

    async fn exists(&self, path: &str) -> bool {
        match self.resolve_path(path) {
            Ok(p) => p.exists(),
            Err(_) => false,
        }
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<String>, FileSystemError> {
        let dir_path = if path.is_empty() || path == "/" {
            self.root.clone()
        } else {
            self.resolve_path(path)?
        };

        if !dir_path.is_dir() {
            return Err(FileSystemError::NotFound(path.to_string()));
        }

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&dir_path).await?;

        while let Some(entry) = dir.next_entry().await? {
            let file_name = entry.file_name();
            entries.push(file_name.to_string_lossy().to_string());
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_directory_filesystem() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"test content").unwrap();

        let fs = DirectoryFileSystem::new(temp_dir.path()).unwrap();

        assert!(fs.exists("test.txt").await);
        assert!(!fs.exists("nonexistent.txt").await);

        let content = fs.read_file("test.txt").await.unwrap();
        assert_eq!(content, b"test content");
    }

    #[tokio::test]
    async fn test_directory_listing() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), b"").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), b"").unwrap();

        let fs = DirectoryFileSystem::new(temp_dir.path()).unwrap();
        let entries = fs.list_dir("").await.unwrap();

        assert!(entries.contains(&"file1.txt".to_string()));
        assert!(entries.contains(&"file2.txt".to_string()));
    }
}

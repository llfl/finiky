use async_trait::async_trait;
use std::path::Path;

pub mod directory;
pub mod tarfs;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Archive error: {0}")]
    Archive(String),
}

#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Read a file from the filesystem
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, FileSystemError>;

    /// Check if a file exists
    async fn exists(&self, path: &str) -> bool;

    /// List files in a directory
    #[allow(dead_code)]
    async fn list_dir(&self, path: &str) -> Result<Vec<String>, FileSystemError>;
}

/// Create a FileSystem from a path (directory or tar.gz file)
pub fn create_filesystem<P: AsRef<Path>>(path: P) -> Result<Box<dyn FileSystem>, FileSystemError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(FileSystemError::NotFound(
            path.to_string_lossy().to_string(),
        ));
    }

    if path.is_dir() {
        Ok(Box::new(directory::DirectoryFileSystem::new(path)?))
    } else if path.extension().and_then(|s| s.to_str()) == Some("gz") {
        Ok(Box::new(tarfs::TarFileSystem::new(path)?))
    } else {
        Err(FileSystemError::InvalidPath(
            "Path must be a directory or .tar.gz file".to_string(),
        ))
    }
}

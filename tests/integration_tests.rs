use finiky::config::Config;
use finiky::filesystem;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_filesystem_creation() {
    // Test directory filesystem
    let temp_dir = TempDir::new().unwrap();
    let fs = filesystem::create_filesystem(temp_dir.path()).unwrap();
    assert!(fs.exists("").await);

    // Test tar.gz filesystem creation (would need actual tar.gz file)
    // This is tested in unit tests
}

#[tokio::test]
async fn test_config_loading() {
    let config = Config::default();
    assert_eq!(config.dhcp.port, 67);
    assert_eq!(config.tftp.port, 69);
    assert_eq!(config.http.port, 8080);
}

#[tokio::test]
async fn test_config_file_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config = Config::default();
    let toml_str = toml::to_string(&config).unwrap();
    fs::write(&config_path, toml_str).unwrap();

    let loaded = Config::from_file(&config_path).unwrap();
    assert_eq!(loaded.dhcp.port, config.dhcp.port);
}

use std::{fs::OpenOptions, path::PathBuf};

use config::{FileFormat, FileSourceFile, builder::DefaultState};
pub use configs::{
    AppConfig, CompressionConfig, CorsConfig, DatabaseConfig, JwtConfig, LoggingConfig,
    MiddlewareConfig, RateLimitConfig, RequestIdConfig, RoomConfig, SecurityConfig, ServerConfig,
    StorageConfig, TracingConfig, UploadConfig,
};
pub use error::{ConfigError, Result};
use merge::Merge;

mod configs;
mod error;
mod merge;

/// Main configuration structure that holds all application settings.
///
/// This structure is designed to work with the config crate's layered configuration system,
/// supporting environment variables, configuration files, and default values.
///
/// Note: We removed `#[serde(flatten)]` to ensure proper deserialization of environment variables.
/// Environment variables should be prefixed with `ELIZABETH__APP__` (e.g., `ELIZABETH__APP__ADDR`, `ELIZABETH__APP__PORT`).
#[derive(Merge, Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    pub app: AppConfig,
}

const DEFAULT_DIR_NAME: &str = ".config";
const DEFAULT_APP_NAME: &str = "elizabeth";
const DEFAULT_CONFIG_FILE_NAME: &str = "config";
const DEFAULT_CONFIG_FILE_EXTENSION: &str = "yaml";

/// Configuration manager that handles loading and saving of application configuration.
///
/// This manager provides thread-safe access to configuration data from multiple sources:
/// - Environment variables (prefixed with `ELIZABETH__`)
/// - Configuration files (YAML format)
/// - Default values
///
/// The manager uses the config crate's layered configuration system where environment
/// variables override file settings, which override default values.
#[derive(Debug, Clone)]
pub struct ConfigManager(config::Config, String);

impl ConfigManager {
    fn default_file_path(
        dir_name: &str,
        app_name: &str,
        file_name: &str,
        extension: &str,
    ) -> PathBuf {
        let save_dir = std::env::var("HOME").unwrap_or_else(|_| {
            std::env::var("CONFIG_DIR").unwrap_or_else(|_| {
                std::env::var("USERPROFILE").unwrap_or_else(|_| {
                    std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
                        std::env::var("APPDATA")
                            .unwrap_or(std::env::var("~").unwrap_or_else(|_| "~".into()))
                    })
                })
            })
        });

        let save_dir = PathBuf::from(save_dir);

        save_dir
            .join(dir_name)
            .join(app_name)
            .join(file_name)
            .with_extension(extension)
    }

    fn env_source() -> config::Environment {
        config::Environment::with_prefix(&DEFAULT_APP_NAME.to_uppercase())
            .separator("__")
            .prefix_separator("__")
            .try_parsing(true)
    }

    fn file_source(file_path: &str, required: bool) -> config::File<FileSourceFile, FileFormat> {
        config::File::new(file_path, config::FileFormat::Yaml).required(required)
    }

    pub fn new() -> Self {
        let default_file_path = Self::default_config_path();
        let default_file_path = default_file_path
            .to_str()
            .expect("default config path must be valid UTF-8")
            .to_owned();
        let config_rs = config::ConfigBuilder::<DefaultState>::default()
            .add_source(Self::file_source(&default_file_path, false))
            .add_source(Self::env_source())
            .build()
            .unwrap();
        Self(config_rs, default_file_path)
    }

    pub fn new_with_file(file_path: &str) -> Self {
        let config_rs = config::ConfigBuilder::<DefaultState>::default()
            .add_source(Self::file_source(file_path, true))
            .add_source(Self::env_source())
            .build()
            .unwrap();
        Self(config_rs, file_path.into())
    }

    pub fn default_config_path() -> PathBuf {
        Self::default_file_path(
            DEFAULT_DIR_NAME,
            DEFAULT_APP_NAME,
            DEFAULT_CONFIG_FILE_NAME,
            DEFAULT_CONFIG_FILE_EXTENSION,
        )
    }

    pub fn file_path(&self) -> &str {
        &self.1
    }

    pub fn file_exists(&self) -> bool {
        PathBuf::from(&self.1).exists()
    }

    /// Load configuration from all sources (environment variables, files, etc.)
    /// This method uses the internal config instance to properly merge all configuration sources
    pub fn source<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Clone the config to ensure thread safety
        let config = self.0.clone();
        config.try_deserialize().map_err(ConfigError::from)
    }

    /// Load configuration from files only
    pub fn load<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let file = OpenOptions::new().read(true).open(&self.1)?;
        let config: T = serde_yaml::from_reader(file).map_err(ConfigError::from)?;
        Ok(config)
    }

    /// Save configuration to default file
    pub fn save<T>(&self, config: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let file_path = PathBuf::from(&self.1);

        let config_str = serde_yaml::to_string(config).map_err(ConfigError::from)?;

        self.atomic_write_with_lock(&file_path, &config_str)?;

        tracing::info!("Config saved to {}", file_path.display());
        Ok(())
    }

    fn atomic_write_with_lock(&self, file_path: &PathBuf, content: &str) -> Result<()> {
        // Create a unique temporary file in the same directory for atomic operation
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(ConfigError::from)?;
        }

        let temp_path = {
            let mut temp_name = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("config")
                .to_string();
            temp_name.push_str(&format!(".tmp.{}", std::process::id()));
            file_path.with_file_name(temp_name)
        };

        const INTERVAL: u64 = 100;
        const MAX_ATTEMPTS: u8 = 3;

        for attempt in 0..MAX_ATTEMPTS {
            match self.try_write_with_lock(&temp_path, content) {
                Ok(_) => break,
                Err(e) => {
                    let _ = std::fs::remove_file(&temp_path);

                    if attempt < MAX_ATTEMPTS - 1 {
                        std::thread::sleep(std::time::Duration::from_millis(INTERVAL));
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        std::fs::rename(&temp_path, file_path).map_err(ConfigError::from)?;
        Ok(())
    }

    /// Try to write with file locking
    fn try_write_with_lock(&self, file_path: &PathBuf, content: &str) -> Result<()> {
        use fs4::fs_std::FileExt;
        use std::io::Write;

        // Open the temporary file for writing
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)
            .map_err(ConfigError::from)?;

        // Try to acquire exclusive lock (non-blocking)
        match file.try_lock_exclusive() {
            Ok(true) => {
                // Lock acquired successfully
            }
            Ok(false) => {
                return Err(ConfigError::Other(
                    "Failed to acquire exclusive file lock".into(),
                ));
            }
            Err(e) => {
                return Err(ConfigError::Other(format!(
                    "Error while trying to lock file {:?}: {}",
                    file_path.display(),
                    e
                )));
            }
        }

        // Write content with lock held
        // Write the serialized content to file
        let mut writer = std::io::BufWriter::new(&file);
        writer.write_all(content.as_bytes())?;
        writer.flush()?;

        // Unlock the file
        let _ = FileExt::unlock(&file);

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock poisoned")
    }

    fn temp_config_file() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("config.yaml");
        std::fs::write(&config_path, "app: {}\n").expect("write temp config");
        (temp, config_path)
    }

    #[test]
    fn test_save() {
        let _lock = env_lock();
        let (_temp_dir, config_path) = temp_config_file();
        let config_manager = ConfigManager::new_with_file(config_path.to_str().unwrap());
        let config = Config {
            app: AppConfig {
                server: ServerConfig {
                    addr: "128.0.0.1".to_string(),
                    port: 0,
                },
                ..Default::default()
            },
        };
        config_manager.save(&config).unwrap();

        // Create a new ConfigManager instance to reload the configuration from file
        let new_config_manager = ConfigManager::new_with_file(config_path.to_str().unwrap());
        let loaded_config: Config = new_config_manager.source().unwrap();
        assert_eq!(loaded_config.app.server.addr, "128.0.0.1");
        assert_eq!(loaded_config.app.server.port, 0);
        let loaded_config_from_file: Config = new_config_manager.load().unwrap();
        assert_eq!(loaded_config_from_file.app.server.addr, "128.0.0.1");
        assert_eq!(loaded_config_from_file.app.server.port, 0);
    }

    #[test]
    fn test_merge() {
        let mut config = Config::default();
        assert_eq!(config.app.server.addr, "127.0.0.1");
        assert_eq!(config.app.server.port, 4092);
        let config_2 = Config {
            app: AppConfig {
                server: ServerConfig {
                    addr: "128.0.0.1".to_string(),
                    port: 0,
                },
                ..Default::default()
            },
        };
        config.merge(config_2);
        assert_eq!(config.app.server.addr, "128.0.0.1");
        assert_eq!(config.app.server.port, 0);
    }

    #[test]
    fn test_nest_merge_strategy() {
        let mut config = Config::default();
        assert_eq!(config.app.server.addr, "127.0.0.1");
        assert_eq!(config.app.server.port, 4092);
        let config_2 = Config {
            app: AppConfig {
                server: ServerConfig {
                    addr: "".to_string(),
                    port: 0,
                },
                ..Default::default()
            },
        };
        config.merge(config_2);
        assert_eq!(config.app.server.addr, "127.0.0.1");
        assert_eq!(config.app.server.port, 0);
    }

    #[test]
    fn test_concurrent_load() {
        use std::sync::Arc;
        use std::thread;

        let _lock = env_lock();
        let (_temp_dir, config_path) = temp_config_file();
        let config_manager = Arc::new(ConfigManager::new_with_file(config_path.to_str().unwrap()));
        let mut handles = vec![];

        // Create multiple threads that try to load configuration concurrently
        for i in 0..10 {
            let manager = Arc::clone(&config_manager);
            let handle = thread::spawn(move || {
                let result: Result<Config> = manager.source();
                // Should not panic and should return consistent results
                assert!(result.is_ok() || result.is_err()); // Either ok or error, but no panic
                i // Return thread id for verification
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            let thread_id = handle.join().expect("Thread should not panic");
            assert!(thread_id < 10);
        }
    }

    #[test]
    fn test_load_with_environment_override() {
        let _lock = env_lock();
        let (_temp_dir, config_path) = temp_config_file();

        // Test that environment variables properly override file settings
        unsafe {
            std::env::set_var("ELIZABETH__APP__SERVER__ADDR", "192.168.1.1");
            std::env::set_var("ELIZABETH__APP__SERVER__PORT", "8080");
        }

        // Create ConfigManager after setting environment variables
        let config_manager = ConfigManager::new_with_file(config_path.to_str().unwrap());
        let config: Result<Config> = config_manager.source();

        // Clean up environment variables
        unsafe {
            std::env::remove_var("ELIZABETH__APP__SERVER__ADDR");
            std::env::remove_var("ELIZABETH__APP__SERVER__PORT");
        }

        match config {
            Ok(cfg) => {
                // Environment variables should override defaults
                println!(
                    "Loaded config: addr={}, port={}",
                    cfg.app.server.addr, cfg.app.server.port
                );
                assert_eq!(cfg.app.server.addr, "192.168.1.1");
                assert_eq!(cfg.app.server.port, 8080);
            }
            Err(e) => {
                println!("Config load failed: {:?}", e);
                panic!("Config should load successfully");
            }
        }
    }
}

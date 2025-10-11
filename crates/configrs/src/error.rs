use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_yaml::Error),
    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 房间上传文件类型策略模式
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default, sqlx::Type,
)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub enum UploadFileTypeMode {
    /// 不限制上传文件类型
    #[default]
    Any,
    /// 仅允许列出的扩展名
    Allow,
    /// 拒绝列出的扩展名
    Deny,
}

impl From<String> for UploadFileTypeMode {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "allow" => UploadFileTypeMode::Allow,
            "deny" => UploadFileTypeMode::Deny,
            _ => UploadFileTypeMode::Any,
        }
    }
}

impl UploadFileTypeMode {
    /// sqlx `Any` 驱动不支持枚举 Type 绑定，写路径统一以字符串参数落库。
    pub fn as_db_str(&self) -> &'static str {
        match self {
            UploadFileTypeMode::Any => "any",
            UploadFileTypeMode::Allow => "allow",
            UploadFileTypeMode::Deny => "deny",
        }
    }
}

/// 房间级上传文件类型策略。扩展名统一为小写、不含点（如 "pdf"）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct UploadFileTypePolicy {
    pub mode: UploadFileTypeMode,
    #[cfg_attr(feature = "typescript-export", ts(type = "Array<string>"))]
    pub extensions: Vec<String>,
}

impl UploadFileTypePolicy {
    /// 文件名是否允许上传。策略在服务端强制，是真实的信任边界。
    pub fn permits(&self, file_name: &str) -> bool {
        let extension = extension_of(file_name);
        match self.mode {
            UploadFileTypeMode::Any => true,
            UploadFileTypeMode::Allow => {
                matches!(&extension, Some(ext) if self.extensions.iter().any(|allowed| allowed == ext))
            }
            UploadFileTypeMode::Deny => {
                !matches!(&extension, Some(ext) if self.extensions.iter().any(|denied| denied == ext))
            }
        }
    }
}

/// 提取文件名最后一段的小写扩展名（不含点；无扩展名返回 None）。
/// 先按路径分隔符切段，避免 "a/b.txt" 这类带路径的原始名误判。
pub fn extension_of(file_name: &str) -> Option<String> {
    let base_name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let (_, extension) = base_name.rsplit_once('.')?;
    let extension = extension.trim();
    if extension.is_empty() {
        None
    } else {
        Some(extension.to_ascii_lowercase())
    }
}

pub const MAX_UPLOAD_FILE_TYPE_EXTENSIONS: usize = 64;
pub const MAX_UPLOAD_FILE_TYPE_EXTENSION_LEN: usize = 16;

/// 扩展名列表校验失败。Display 产出稳定的英文消息，前端按前缀映射到 i18n。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadFilePolicyError {
    EmptyExtensions,
    TooManyExtensions,
    InvalidExtension(String),
}

impl std::fmt::Display for UploadFilePolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadFilePolicyError::EmptyExtensions => write!(
                f,
                "upload_file_type.extensions must not be empty for allow or deny mode"
            ),
            UploadFilePolicyError::TooManyExtensions => write!(
                f,
                "upload_file_type.extensions supports at most {MAX_UPLOAD_FILE_TYPE_EXTENSIONS} entries"
            ),
            UploadFilePolicyError::InvalidExtension(raw) => {
                write!(f, "Invalid file type extension: {raw}")
            }
        }
    }
}

/// 规范化扩展名列表：小写、去前导点、去重保序；校验字符集与数量。
pub fn normalize_upload_file_extensions(
    raw: &[String],
) -> Result<Vec<String>, UploadFilePolicyError> {
    if raw.len() > MAX_UPLOAD_FILE_TYPE_EXTENSIONS {
        return Err(UploadFilePolicyError::TooManyExtensions);
    }

    let mut normalized: Vec<String> = Vec::with_capacity(raw.len());
    for entry in raw {
        let extension = entry.trim().trim_start_matches('.').to_ascii_lowercase();
        let is_valid = !extension.is_empty()
            && extension.len() <= MAX_UPLOAD_FILE_TYPE_EXTENSION_LEN
            && extension.chars().all(|c| c.is_ascii_alphanumeric());
        if !is_valid {
            return Err(UploadFilePolicyError::InvalidExtension(entry.clone()));
        }
        if !normalized.contains(&extension) {
            normalized.push(extension);
        }
    }
    Ok(normalized)
}

/// 校验并规范化完整策略：Any 模式清空列表；Allow/Deny 模式要求列表非空。
pub fn normalize_upload_file_type(
    policy: UploadFileTypePolicy,
) -> Result<UploadFileTypePolicy, UploadFilePolicyError> {
    let extensions = match policy.mode {
        UploadFileTypeMode::Any => Vec::new(),
        UploadFileTypeMode::Allow | UploadFileTypeMode::Deny => {
            let extensions = normalize_upload_file_extensions(&policy.extensions)?;
            if extensions.is_empty() {
                return Err(UploadFilePolicyError::EmptyExtensions);
            }
            extensions
        }
    };
    Ok(UploadFileTypePolicy {
        mode: policy.mode,
        extensions,
    })
}

/// 上传被策略拒绝时的稳定错误消息；前端按前缀映射为本地化文案。
pub fn upload_file_type_violation(file_name: &str) -> String {
    format!("File type not allowed by room policy: {file_name}")
}

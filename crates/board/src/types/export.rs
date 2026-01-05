//! TypeScript 类型生成模块
//!
//! 此模块为所有 API 相关类型实现 TS trait，用于自动生成 TypeScript 类型定义。

#[cfg(feature = "typescript-export")]
use ts_rs::TS;

// 为 RoomStatus 实现 TS
#[cfg(feature = "typescript-export")]
impl TS for super::RoomStatus {
    fn decl() -> String {
        "enum RoomStatus { Open = 0, Lock = 1, Close = 2 }".to_string()
    }

    fn name() -> String {
        "RoomStatus".to_string()
    }

    fn dependencies() -> Vec<ts_rs::Dependency> {
        vec![]
    }
}

// 为 ContentType 实现 TS
#[cfg(feature = "typescript-export")]
impl TS for super::ContentType {
    fn decl() -> String {
        "enum ContentType { Text = 0, Image = 1, File = 2, Url = 3 }".to_string()
    }

    fn name() -> String {
        "ContentType".to_string()
    }

    fn dependencies() -> Vec<ts_rs::Dependency> {
        vec![]
    }
}

// 为 ChunkStatus 实现 TS
#[cfg(feature = "typescript-export")]
impl TS for super::ChunkStatus {
    fn decl() -> String {
        "enum ChunkStatus { Pending = 0, InProgress = 1, Completed = 2, Failed = 3 }".to_string()
    }

    fn name() -> String {
        "ChunkStatus".to_string()
    }

    fn dependencies() -> Vec<ts_rs::Dependency> {
        vec![]
    }
}

// 为 UploadStatus 实现 TS
#[cfg(feature = "typescript-export")]
impl TS for super::UploadStatus {
    fn decl() -> String {
        "enum UploadStatus { Reserved = 0, InProgress = 1, Completed = 2, Expired = 3 }".to_string()
    }

    fn name() -> String {
        "UploadStatus".to_string()
    }

    fn dependencies() -> Vec<ts_rs::Dependency> {
        vec![]
    }
}

// 导出函数：生成所有类型的 TypeScript 定义
#[cfg(feature = "typescript-export")]
pub fn export_types() -> String {
    let mut types = String::new();

    // 添加导入语句
    types.push_str("// Auto-generated TypeScript types from Rust\n");
    types.push_str("// DO NOT EDIT MANUALLY\n\n");
    types.push_str("import { z } from 'zod';\n\n");

    // 导出所有类型
    types.push_str(&<super::Room as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::RoomStatus as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::RoomContent as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::ContentType as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::RoomPermission as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::RoomToken as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::ChunkUploadRequest as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::ChunkUploadResponse as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::RoomChunkUpload as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::CreateRefreshTokenRequest as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::RefreshTokenRequest as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::RefreshTokenResponse as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::RoomRefreshToken as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::UploadFileDescriptor as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::UploadStatus as TS>::decl());
    types.push_str("\n\n");
    types.push_str(&<super::ChunkStatus as TS>::decl());

    types
}

/// 导出所有类型到指定目录
#[cfg(feature = "typescript-export")]
pub fn write_types_to_file(output_path: &std::path::Path) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;

    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let types = export_types();
    let mut file = File::create(output_path)?;
    file.write_all(types.as_bytes())?;
    file.write_all(b"\n")?;

    Ok(())
}

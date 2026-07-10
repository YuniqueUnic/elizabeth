pub fn main() {
    println!("cargo:rerun-if-env-changed=DYNAMIC");
    println!("cargo:rerun-if-env-changed=ELIZABETH_SKIP_WEB_BUILD");
    if std::env::var("DYNAMIC").is_ok() {
        println!("cargo:rustc-cfg=feature=\"dynamic\"");
    }

    let denied_consts = [
        "CARGO_MANIFEST_DIR",
        "PROJECT_NAME",
        "CARGO_TREE",
        "GIT_STATUS_FILE",
        "BUILD_TARGET_ARCH",
        "COMMIT_EMAIL",
        "COMMIT_AUTHOR",
        "COMMIT_DATE_2822",
        "COMMIT_DATE",
        "CLAP_LONG_VERSION",
        "BUILD_TIME_2822",
    ];

    use shadow_rs::ShadowBuilder;
    ShadowBuilder::builder()
        .deny_const(denied_consts.into())
        .build()
        .unwrap();

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    // 生成补全
    #[cfg(feature = "completions")]
    {
        // 生成补全脚本
        if let Err(e) = generate_completions() {
            eprintln!("Failed to generate completions: {}", e);
        }
    }

    // 生成 TypeScript 类型定义
    #[cfg(feature = "typescript-export")]
    {
        if let Err(e) = generate_frontend_bindings() {
            eprintln!("Failed to generate frontend bindings: {}", e);
        }
    }

    if let Err(error) = ensure_embedded_frontend() {
        panic!("Failed to prepare embedded frontend assets: {error:#}");
    }
}

const FRONTEND_INPUTS: &[&str] = &[
    "web/app",
    "web/api",
    "web/components",
    "web/hooks",
    "web/i18n",
    "web/lib",
    "web/messages",
    "web/public",
    "web/styles",
    "web/types/generated",
    "web/package.json",
    "web/bun.lock",
    "web/next.config.mjs",
    "web/postcss.config.mjs",
    "web/tsconfig.json",
    "web/proxy.ts",
];

fn ensure_embedded_frontend() -> anyhow::Result<()> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let workspace_root = manifest_dir
        .parent()
        .and_then(|parent| parent.parent())
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to determine workspace root from CARGO_MANIFEST_DIR")
        })?;
    let web_dir = workspace_root.join("web");
    let embedded_dir = manifest_dir.join("web-out");

    for input in FRONTEND_INPUTS {
        register_rerun_path(&workspace_root.join(input))?;
    }

    if !web_dir.exists() {
        eprintln!(
            "[board/build.rs] skipping embedded frontend rebuild because {:?} is unavailable in this build context",
            web_dir
        );
        return Ok(());
    }

    if should_skip_embedded_frontend_build() {
        if !has_embedded_frontend_assets(&embedded_dir) {
            anyhow::bail!(
                "`ELIZABETH_SKIP_WEB_BUILD` is set, but {:?} is empty or missing. Run `bun run build:embedded` in `web/` first.",
                embedded_dir
            );
        }
        eprintln!(
            "[board/build.rs] skipping `bun run build:embedded` because ELIZABETH_SKIP_WEB_BUILD is set"
        );
        return Ok(());
    }

    if !embedded_frontend_build_needed(workspace_root, &embedded_dir)? {
        return Ok(());
    }

    eprintln!(
        "[board/build.rs] running `bun run build:embedded` in {:?}",
        web_dir
    );

    let status = std::process::Command::new("bun")
        .arg("run")
        .arg("build:embedded")
        .current_dir(&web_dir)
        .status()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => anyhow::anyhow!(
                "`bun` was not found in PATH. Install Bun or set ELIZABETH_SKIP_WEB_BUILD=1 if you intentionally want to reuse existing embedded assets."
            ),
            _ => anyhow::Error::new(error),
        })?;

    if !status.success() {
        anyhow::bail!("`bun run build:embedded` failed with status {status}");
    }

    if !has_embedded_frontend_assets(&embedded_dir) {
        anyhow::bail!(
            "`bun run build:embedded` completed, but {:?} is still empty or missing",
            embedded_dir
        );
    }

    Ok(())
}

fn should_skip_embedded_frontend_build() -> bool {
    matches!(
        std::env::var("ELIZABETH_SKIP_WEB_BUILD"),
        Ok(value) if matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
    )
}

fn embedded_frontend_build_needed(
    workspace_root: &std::path::Path,
    embedded_dir: &std::path::Path,
) -> anyhow::Result<bool> {
    if !has_embedded_frontend_assets(embedded_dir) {
        return Ok(true);
    }

    let newest_input = FRONTEND_INPUTS
        .iter()
        .filter_map(|path| newest_modified_at(&workspace_root.join(path)).transpose())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max();

    let newest_output = newest_modified_at(embedded_dir)?;

    Ok(match (newest_input, newest_output) {
        (_, None) => true,
        (Some(input), Some(output)) => input > output,
        (None, Some(_)) => false,
    })
}

fn register_rerun_path(path: &std::path::Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    println!("cargo:rerun-if-changed={}", path.display());
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            register_rerun_path(&entry.path())?;
        }
    }

    Ok(())
}

fn newest_modified_at(path: &std::path::Path) -> anyhow::Result<Option<std::time::SystemTime>> {
    if !path.exists() {
        return Ok(None);
    }

    let metadata = std::fs::metadata(path)?;
    let mut newest = metadata.modified().ok();

    if metadata.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if let Some(child_modified) = newest_modified_at(&entry.path())? {
                newest = Some(match newest {
                    Some(current) if current >= child_modified => current,
                    _ => child_modified,
                });
            }
        }
    }

    Ok(newest)
}

fn has_embedded_frontend_assets(path: &std::path::Path) -> bool {
    path.exists()
        && path.is_dir()
        && std::fs::read_dir(path)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
}

// 包含 CLI 定义
#[cfg(feature = "completions")]
include!("src/cmd/cli.rs");

/// 生成补全脚本
#[cfg(feature = "completions")]
fn generate_completions() -> anyhow::Result<()> {
    use clap::{CommandFactory, ValueEnum};
    use clap_complete::Shell;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    let outdir = env::var_os("OUT_DIR")
        .ok_or("OUT_DIR not set")
        .map_err(|e| anyhow::anyhow!("Failed to get OUT_DIR: {}", e))?;
    let outdir_path = Path::new(&outdir);

    let app_name = env!("CARGO_PKG_NAME");

    let completions_dir = outdir_path.join("completions");
    fs::create_dir_all(&completions_dir)?;

    let completions_rs = outdir_path.join("completions.rs");
    let mut file = File::create(&completions_rs)?;

    writeln!(file, "// Generated shell completions")?;
    writeln!(file, "// This file is automatically generated by build.rs")?;
    writeln!(file, "// Do not edit this file manually")?;
    writeln!(file)?;

    for &shell in Shell::value_variants() {
        let shell_name = match shell {
            Shell::Bash => "bash",
            Shell::Elvish => "elvish",
            Shell::Fish => "fish",
            Shell::PowerShell => "powershell",
            Shell::Zsh => "zsh",
            _ => "bash",
        };

        let mut cmd = Cli::command();

        let file_path = clap_complete::generate_to(shell, &mut cmd, app_name, &completions_dir)?;

        let content = fs::read_to_string(&file_path)?;

        writeln!(file, "/// Completion script for {}", shell_name)?;
        writeln!(
            file,
            "pub const {}_COMPLETION: &str = r#\"{}\"#;",
            shell_name.to_uppercase(),
            content
        )?;
        writeln!(file)?;
    }

    Ok(())
}

/// 生成 TypeScript 类型定义
#[cfg(feature = "typescript-export")]
fn generate_frontend_bindings() -> anyhow::Result<()> {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    // 获取工作区根目录（elizabeth 项目根目录）
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Failed to get workspace root"))?;

    // 输出目录：web/types/generated/
    let output_dir = workspace_root.join("web/types/generated");
    fs::create_dir_all(&output_dir)?;

    // 清理旧的生成产物（避免删除类型后残留旧文件）
    clean_generated_dir(&output_dir)?;

    // 生成 ts-rs types
    board_protocol::codegen::export_ts_types_to(&output_dir)?;

    // 写入入口文件（前端从这里 import）
    write_ts_index(&output_dir)?;

    // 生成 JSON Schema（供前端消费）
    let schema_json = board_protocol::codegen::api_schema_json_pretty()?;
    let schema_path = output_dir.join("api.schema.json");
    fs::write(&schema_path, schema_json)?;

    println!("cargo:rerun-if-env-changed=ELIZABETH_CODEGEN_VERBOSE");
    if env::var("ELIZABETH_CODEGEN_VERBOSE").is_ok() {
        eprintln!("[codegen] generated TypeScript types into {:?}", output_dir);
        eprintln!("[codegen] generated JSON schema into {:?}", schema_path);
    }

    // 通知 cargo 在以下文件变化时重新运行
    println!("cargo:rerun-if-changed=../board-protocol/src/constants.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/codegen.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/dto/mod.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/dto/admin.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/dto/token.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/dto/rooms.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/dto/content.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/dto/chunked_upload.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/dto/auth.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/models/mod.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/models/room/mod.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/models/room/content.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/models/room/chunk_upload.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/models/room/permission.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/models/room/refresh_token.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/models/room/token.rs");
    println!("cargo:rerun-if-changed=../board-protocol/src/models/room/upload_reservation.rs");

    Ok(())
}

#[cfg(feature = "typescript-export")]
fn clean_generated_dir(dir: &std::path::Path) -> anyhow::Result<()> {
    use std::fs;

    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if matches!(ext, "ts" | "json") {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

#[cfg(feature = "typescript-export")]
fn write_ts_index(output_dir: &std::path::Path) -> anyhow::Result<()> {
    use std::env;
    use std::fs;

    let index_file = output_dir.join("api.types.ts");
    let mut content = String::new();
    content.push_str("// Auto-generated TypeScript types from Rust backend\n");
    content.push_str("// This file is automatically generated by crates/board/build.rs\n");
    content.push_str("// DO NOT EDIT THIS FILE MANUALLY\n");
    content.push_str("//\n");
    content.push_str("// To regenerate types, run:\n");
    content.push_str("//   cargo build --package elizabeth-board --features typescript-export\n");
    content.push_str("//\n\n");

    content.push_str("// Re-export all generated types\n");
    for &type_name in board_protocol::codegen::exported_ts_type_names() {
        content.push_str(&format!("export * from './{type_name}';\n"));
    }

    fs::write(&index_file, content)?;
    println!("cargo:rerun-if-env-changed=ELIZABETH_CODEGEN_VERBOSE");
    if env::var("ELIZABETH_CODEGEN_VERBOSE").is_ok() {
        eprintln!("[codegen] generated TS index {:?}", index_file);
    }
    Ok(())
}

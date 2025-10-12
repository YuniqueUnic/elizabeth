use crate::build;

pub const INFO: &str = shadow_rs::formatcp!(
    r#"
version: {}
branch: {}
commit_short_hash: {}
build_time: {}
rustup toolchain: {}
git clean status: {}
"#,
    build::PKG_VERSION,
    build::BRANCH,
    build::SHORT_COMMIT,
    build::BUILD_TIME_3339,
    build::RUST_CHANNEL,
    build::GIT_CLEAN
);

pub fn init() {
    println!("{}", INFO);
}

use crate::cli::args::UpdateCommands;
use anyhow::{bail, Context, Result};
use self_update::cargo_crate_version;

/// 发布源仓库（init 时替换占位符；self_update 需要编译期常量，
/// 是全仓库唯一允许占位符的源码文件，init 按文件白名单定点替换）
pub const REPO_OWNER: &str = "{{REPO_OWNER}}";
pub const REPO_NAME: &str = "{{REPO_NAME}}";

/// 执行更新命令
pub fn execute(cmd: UpdateCommands) -> Result<()> {
    match cmd {
        UpdateCommands::Check => check_update(),
        UpdateCommands::Apply => apply_update(),
    }
}

/// 占位符是否已被 init 替换（模板态返回 false）
fn release_source_ready() -> bool {
    !REPO_OWNER.contains("{{") && !REPO_NAME.contains("{{")
}

fn ensure_ready() -> Result<()> {
    if !release_source_ready() {
        bail!("发布源仓库未配置（仍为模板占位值），请先运行 `cargo run --bin init` 填写仓库地址");
    }
    Ok(())
}

/// 检查是否有新版本可用（公开仓库免 token）
fn check_update() -> Result<()> {
    println!("正在检查更新...");
    ensure_ready()?;

    let current = cargo_crate_version!();
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .context("无法配置 GitHub Release 查询")?
        .fetch()
        .context("无法从 GitHub 获取版本列表")?;

    if releases.is_empty() {
        println!("未找到任何发布版本");
        return Ok(());
    }

    let latest = releases.latest().expect("非空列表必有最新项");
    println!("当前版本: {current}");
    println!("最新版本: {}", latest.version());

    // 远端版本可能比本地旧（如手动装了 dev 版），只有更大才提示更新
    if self_update::version::bump_is_greater(current, latest.version())
        .context("版本号不是合法 semver，无法比较")?
    {
        println!("发现新版本 {} (当前: {current})", latest.version());
        println!("运行 `{} update apply` 来更新", env!("CARGO_BIN_NAME"));
    } else {
        println!("已经是最新版本");
    }
    Ok(())
}

/// 下载并安装最新版本
fn apply_update() -> Result<()> {
    println!("正在检查并更新...");
    ensure_ready()?;

    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(env!("CARGO_BIN_NAME"))
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()
        .context("无法配置更新器")?
        .update()
        .context("更新失败")?;

    match status {
        self_update::VersionStatus::UpToDate(version) => {
            println!("已经是最新版本 ({version})");
        }
        self_update::VersionStatus::Updated(version) => {
            println!("已更新到版本 {version}");
        }
        // VersionStatus 标注 #[non_exhaustive]，未来新增变体在此兜底
        _ => {
            println!("更新完成");
        }
    }

    Ok(())
}

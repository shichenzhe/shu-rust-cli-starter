use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// 执行安装/卸载命令
pub fn execute(uninstall: bool) -> Result<()> {
    if uninstall {
        do_uninstall()
    } else {
        do_install()
    }
}

/// 安装当前可执行文件到系统 PATH
fn do_install() -> Result<()> {
    let bin = env!("CARGO_BIN_NAME");
    let current_exe = std::env::current_exe().context("无法获取当前可执行文件路径")?;
    let install_dir = get_install_dir()?;
    let dest = install_dir.join(binary_name());

    // 已在安装位置，仅确保 PATH 配置
    if dest.exists() && same_file(&current_exe, &dest) {
        println!("{bin} 已安装在: {}", dest.display());
        ensure_in_path(&install_dir)?;
        return Ok(());
    }

    // 创建目录并复制
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("无法创建目录 {}", install_dir.display()))?;
    fs::copy(&current_exe, &dest)
        .with_context(|| format!("无法复制 {} -> {}", current_exe.display(), dest.display()))?;
    println!("已安装 {bin} -> {}", dest.display());

    // Unix: 设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
    }

    ensure_in_path(&install_dir)?;

    #[cfg(unix)]
    println!("\n安装完成！请在当前终端执行以下命令使其立即生效：\n  source ~/.bashrc");
    #[cfg(windows)]
    println!("\n安装完成！请重新打开终端以使 PATH 生效。");

    Ok(())
}

/// 卸载
fn do_uninstall() -> Result<()> {
    let bin = env!("CARGO_BIN_NAME");
    let install_dir = get_install_dir()?;
    let dest = install_dir.join(binary_name());

    if !dest.exists() {
        bail!("未找到已安装的 {bin} ({})", dest.display());
    }

    fs::remove_file(&dest).with_context(|| format!("无法删除 {}", dest.display()))?;
    println!("已删除 {}", dest.display());

    remove_from_path(&install_dir)?;
    println!("\n卸载完成！请重新打开终端以生效。");

    Ok(())
}

/// 获取二进制文件名（Windows 带 .exe）
fn binary_name() -> String {
    if cfg!(windows) {
        format!("{}.exe", env!("CARGO_BIN_NAME"))
    } else {
        env!("CARGO_BIN_NAME").to_string()
    }
}

/// 比较两个路径是否指向同一文件
fn same_file(a: &Path, b: &Path) -> bool {
    a.canonicalize().ok() == b.canonicalize().ok()
}

// ============================================================
// 平台特定：安装目录
// ============================================================

#[cfg(unix)]
fn get_install_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".local").join("bin"))
        .context("无法获取用户主目录")
}

#[cfg(windows)]
fn get_install_dir() -> Result<PathBuf> {
    dirs::data_local_dir()
        .map(|d| d.join(env!("CARGO_PKG_NAME")))
        .context("无法获取本地数据目录")
}

// ============================================================
// Unix: PATH 管理（修改 shell rc 文件）
// ============================================================

#[cfg(unix)]
fn ensure_in_path(_dir: &Path) -> Result<()> {
    // unix 安装目录固定为 ~/.local/bin，此处写死检测与写入；
    // 参数仅为与 windows 版（写注册表）签名一致
    let home = dirs::home_dir().context("无法获取用户主目录")?;
    let path_export = r#"export PATH="$HOME/.local/bin:$PATH""#;

    let rc_files = [".bashrc", ".zshrc", ".bash_profile", ".profile"];
    let mut any_modified = false;

    for name in &rc_files {
        let path = home.join(name);
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(&path).with_context(|| format!("无法读取 ~/{name}"))?;

        // 已包含 .local/bin 路径
        if content.contains(".local/bin") {
            println!("  {name} 已包含 .local/bin");
            continue;
        }

        // 追加 PATH 设置
        fs::write(&path, format!("{}\n{}\n", content.trim_end(), path_export))
            .with_context(|| format!("无法写入 ~/{name}"))?;
        println!("  已添加 PATH 到 ~/{name}");
        any_modified = true;
    }

    if !any_modified {
        println!("  提示：请确保 ~/.local/bin 在你的 PATH 中");
    }

    Ok(())
}

#[cfg(unix)]
fn remove_from_path(_dir: &Path) -> Result<()> {
    let home = dirs::home_dir().context("无法获取用户主目录")?;
    let target = r#"export PATH="$HOME/.local/bin:$PATH""#;

    for name in [".bashrc", ".zshrc", ".bash_profile", ".profile"] {
        let path = home.join(name);
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(&path).with_context(|| format!("无法读取 ~/{name}"))?;
        if !content.contains(target) {
            continue;
        }

        let new_content = content.replace(&format!("\n{target}\n"), "\n");
        if new_content != content {
            fs::write(&path, &new_content).with_context(|| format!("无法写入 ~/{name}"))?;
            println!("  已从 ~/{name} 移除 PATH 设置");
        }
    }

    Ok(())
}

// ============================================================
// Windows: PATH 管理（修改注册表）
// ============================================================

#[cfg(windows)]
fn ensure_in_path(dir: &Path) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let dir_str = dir.to_str().context("路径编码错误")?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .context("无法打开注册表 Environment")?;

    let current: String = env.get_value("Path").unwrap_or_default();

    // 检查是否已包含
    if current.split(';').any(|p| p.eq_ignore_ascii_case(dir_str)) {
        println!("  PATH 已包含 {dir_str}");
        return Ok(());
    }

    let new_path = format!("{};{dir_str}", current.trim_end_matches(';'));
    env.set_value("Path", &new_path)
        .context("无法设置 PATH 环境变量")?;
    println!("  已添加 {dir_str} 到用户 PATH");

    Ok(())
}

#[cfg(windows)]
fn remove_from_path(dir: &Path) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let dir_str = dir.to_str().context("路径编码错误")?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .context("无法打开注册表 Environment")?;

    let current: String = env.get_value("Path").unwrap_or_default();
    let new_path: String = current
        .split(';')
        .filter(|p| !p.eq_ignore_ascii_case(dir_str))
        .collect::<Vec<_>>()
        .join(";");

    if new_path != current {
        env.set_value("Path", &new_path)
            .context("无法更新 PATH 环境变量")?;
        println!("  已从 PATH 移除 {dir_str}");
    }

    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn same_file_detects_identity() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        fs::write(&a, b"x").unwrap();
        let b = dir.path().join("b");
        fs::write(&b, b"x").unwrap();
        assert!(same_file(&a, &a));
        assert!(!same_file(&a, &b));
    }

    #[test]
    fn binary_name_never_empty() {
        assert!(!binary_name().is_empty());
    }
}

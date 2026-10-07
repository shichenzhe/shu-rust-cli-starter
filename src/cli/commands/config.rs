use crate::cli::args::TopConfigCommands;
use crate::config::settings::Settings;
use anyhow::{Context, Result};
use std::io::{BufRead, IsTerminal, Write};

/// 顶层 config：交互式初始化
pub fn execute(command: TopConfigCommands) -> Result<()> {
    match command {
        TopConfigCommands::Init => execute_init(),
    }
}

/// 交互式初始化：demo token 可选（匿名也能用，仅提升速率限制）
fn execute_init() -> Result<()> {
    let path = Settings::config_path()?;
    let mut settings = if path.exists() {
        println!("配置文件已存在（直接回车保留当前值）: {}", path.display());
        Settings::load().context("无法解析现有配置文件，请先修复后再运行 init")?
    } else {
        println!("将创建配置文件: {}", path.display());
        Settings::skeleton()
    };

    if !std::io::stdin().is_terminal() {
        println!("（非交互输入：按 token 顺序逐行提供，空行跳过）");
    }

    let token = prompt_value(
        "GitHub Token（可选，匿名调用限 60 次/小时）",
        Some(configured_hint(&settings.demo.token)),
        true,
    )?;
    if !token.is_empty() {
        settings.demo.token = token;
    }
    settings.save()?;

    println!("\n已保存 {}", path.display());
    Ok(())
}

/// 读取一行输入：secret 且在终端时用 rpassword 不回显；空输入返回空串（保留原值）
fn prompt_value(prompt: &str, hint: Option<&str>, secret: bool) -> Result<String> {
    let suffix = match hint.filter(|h| !h.is_empty()) {
        Some(h) => format!(" [{}]", h),
        None => String::new(),
    };
    if secret && std::io::stdin().is_terminal() {
        let value = rpassword::prompt_password(format!("{}{}: ", prompt, suffix))
            .context("无法读取输入")?;
        Ok(value.trim().to_string())
    } else {
        print!("{}{}: ", prompt, suffix);
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut line)
            .context("无法读取输入")?;
        // 管道输入无回显，补换行避免后续提示挤在同一行
        if !std::io::stdin().is_terminal() {
            println!();
        }
        Ok(line.trim().to_string())
    }
}

/// 敏感字段的当前值提示：只说明是否已配置，不回显内容
fn configured_hint(secret: &str) -> &'static str {
    if secret.is_empty() {
        "未配置"
    } else {
        "已配置，回车保留"
    }
}

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 应用配置（~/.<crate-name>/config.toml；目录名编译期取自包名，
/// init 改名后自动跟随，源码零占位符）
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub demo: DemoSettings,
}

/// demo 域连接配置
#[derive(Debug, Serialize, Deserialize)]
pub struct DemoSettings {
    /// API 根地址（默认 GitHub API，可指向 GitHub Enterprise）
    #[serde(default = "default_demo_base")]
    pub base_url: String,
    /// 访问令牌（可选；匿名调用有 60 次/小时的速率限制）
    #[serde(default)]
    pub token: String,
}

impl Default for DemoSettings {
    fn default() -> Self {
        Self {
            base_url: default_demo_base(),
            token: String::new(),
        }
    }
}

fn default_demo_base() -> String {
    "https://api.github.com".to_string()
}

impl Settings {
    /// 配置目录：~/.<crate-name>
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("无法获取用户主目录")?;
        Ok(home.join(format!(".{}", env!("CARGO_PKG_NAME"))))
    }

    /// 配置文件：~/.<crate-name>/config.toml
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        let content = fs::read_to_string(&path)
            .with_context(|| format!("无法读取配置文件: {}", path.display()))?;
        let settings: Settings = toml::from_str(&content)
            .with_context(|| format!("无法解析配置文件: {}", path.display()))?;
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir).with_context(|| format!("无法创建配置目录: {}", dir.display()))?;
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).context("无法序列化配置")?;
        fs::write(&path, content)
            .with_context(|| format!("无法写入配置文件: {}", path.display()))?;

        // 配置文件可能含凭据（如 token），权限收敛为 0600（仅属主可读写）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("无法设置文件权限: {}", path.display()))?;
        }
        Ok(())
    }

    /// 初始化配置文件骨架（已存在则跳过）
    pub fn init() -> Result<()> {
        let path = Self::config_path()?;
        if path.exists() {
            println!("配置文件已存在: {}", path.display());
            return Ok(());
        }
        Settings::skeleton().save()?;
        println!("配置文件已创建: {}", path.display());
        println!("常用配置:");
        println!(
            "  {} demo config set base_url <API 根地址>",
            env!("CARGO_BIN_NAME")
        );
        println!(
            "  {} demo config set token <GITHUB TOKEN（可选）>",
            env!("CARGO_BIN_NAME")
        );
        Ok(())
    }

    /// 按 section + key 设置配置项
    pub fn set(&mut self, section: &str, key: &str, value: &str) -> Result<()> {
        match (section, key) {
            ("demo", "base_url") => self.demo.base_url = value.to_string(),
            ("demo", "token") => self.demo.token = value.to_string(),
            _ => anyhow::bail!("未知配置项: {}.{}, 可选: base_url, token", section, key),
        }
        Ok(())
    }

    /// 显示指定 section 的配置（敏感字段脱敏）
    pub fn display(&self, section: &str) -> String {
        match section {
            "demo" => format!(
                "[demo]\n  base_url = {}\n  token = {}",
                self.demo.base_url,
                mask(&self.demo.token)
            ),
            _ => format!("[{}]\n  (未知 section)", section),
        }
    }

    /// 构造默认值骨架（不读磁盘，供 init 与测试使用）
    pub fn skeleton() -> Self {
        Settings::default()
    }
}

/// 敏感字段脱敏：保留尾 4 位
fn mask(secret: &str) -> String {
    if secret.len() > 4 {
        format!("****{}", &secret[secret.len() - 4..])
    } else if secret.is_empty() {
        String::new()
    } else {
        "****".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_url_is_github() {
        assert_eq!(DemoSettings::default().base_url, "https://api.github.com");
        assert_eq!(Settings::skeleton().demo.base_url, "https://api.github.com");
    }

    #[test]
    fn set_demo_fields_without_prefix() {
        let mut s = Settings::skeleton();
        s.set("demo", "base_url", "https://ghe.example.com/api/v3")
            .unwrap();
        s.set("demo", "token", "ghp-1234").unwrap();
        assert_eq!(s.demo.base_url, "https://ghe.example.com/api/v3");
        assert_eq!(s.demo.token, "ghp-1234");
    }

    #[test]
    fn set_unknown_key_errors() {
        let mut s = Settings::skeleton();
        assert!(s.set("demo", "nope", "x").is_err());
        assert!(s.set("other", "base_url", "x").is_err());
    }

    #[test]
    fn display_masks_token() {
        let mut s = Settings::skeleton();
        s.set("demo", "token", "ghp-abcd1234").unwrap();
        let out = s.display("demo");
        assert!(out.contains("****1234"));
        assert!(!out.contains("ghp-abcd1234"));
        assert!(out.contains("https://api.github.com"));
    }

    #[test]
    fn toml_roundtrip_keeps_values() {
        let mut s = Settings::skeleton();
        s.set("demo", "token", "tok").unwrap();
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back.demo.token, "tok");
        assert_eq!(back.demo.base_url, "https://api.github.com");
    }
}

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = env!("CARGO_PKG_DESCRIPTION"), long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// 日志级别
    #[arg(short, long, global = true, value_enum, default_value = "warn")]
    pub log_level: LogLevel,
}

#[derive(Subcommand)]
pub enum Command {
    /// 示范域：GitHub 公开 API 查询
    Demo {
        #[command(subcommand)]
        command: DemoCommand,
    },
    /// 安装到系统 PATH
    Install,
    /// 从系统 PATH 卸载
    Uninstall,
    /// 交互式初始化配置（token 不回显）
    Config {
        #[command(subcommand)]
        command: TopConfigCommands,
    },
    /// 检测配置文件与 API 连通性
    Doctor,
    /// 检查并安装更新（GitHub Releases）
    Update {
        #[command(subcommand)]
        command: UpdateCommands,
    },
}

#[derive(Subcommand)]
pub enum DemoCommand {
    /// 列出用户（或组织）的公开仓库
    Repos {
        /// GitHub 用户名或组织名
        owner: String,
        /// CSV 输出
        #[arg(long)]
        csv: bool,
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// 列出仓库的 open issues（自动过滤 PR）
    Issues {
        /// 仓库，格式 owner/repo
        repo: String,
        /// CSV 输出
        #[arg(long)]
        csv: bool,
        /// JSON 输出
        #[arg(long)]
        json: bool,
    },
    /// demo 配置管理（写入 [demo] section）
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

/// 子系统就近 config 命令（demo 用；key 不带 section 前缀）
#[derive(Subcommand)]
pub enum ConfigCommands {
    /// 创建配置文件骨架
    Init,
    /// 查看当前配置（敏感字段脱敏）
    Show,
    /// 设置配置项
    Set { key: String, value: String },
}

/// 顶层 config 命令（交互式初始化全部配置）
#[derive(Subcommand)]
pub enum TopConfigCommands {
    /// 交互式初始化配置
    Init,
}

#[derive(Subcommand)]
pub enum UpdateCommands {
    /// 检查是否有新版本
    Check,
    /// 更新到最新版本
    Apply,
}

#[derive(clap::ValueEnum, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

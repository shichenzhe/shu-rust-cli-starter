use clap::Parser;
use cli::args::{Cli, Command, DemoCommand};

mod cli;
mod config;
mod demo;
mod output;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let level: tracing::Level = cli.log_level.clone().into();
    // 日志统一输出到 stderr：stdout 是命令数据通道（表格 / CSV / JSON，
    // 常被重定向到文件或管道），任何日志混入都会污染输出。
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(level)
        .init();

    match cli.command {
        Command::Demo { command } => dispatch_demo(command)?,
        Command::Install => cli::commands::setup::execute(false)?,
        Command::Uninstall => cli::commands::setup::execute(true)?,
        Command::Config { command } => cli::commands::config::execute(command)?,
        Command::Doctor => cli::commands::doctor::execute()?,
        Command::Update { command } => cli::commands::update::execute(command)?,
    }
    Ok(())
}

fn dispatch_demo(command: DemoCommand) -> anyhow::Result<()> {
    match command {
        DemoCommand::Repos { owner, csv, json } => {
            cli::commands::demo::repos::execute(owner, csv, json)?
        }
        DemoCommand::Issues { repo, csv, json } => {
            cli::commands::demo::issues::execute(repo, csv, json)?
        }
        DemoCommand::Config { command } => cli::commands::demo::config::execute(command)?,
    }
    Ok(())
}

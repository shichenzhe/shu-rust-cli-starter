use crate::cli::args::ConfigCommands;
use crate::config::settings::Settings;
use anyhow::Result;

/// demo config：管 [demo] section（key 不带前缀）
pub fn execute(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Init => Settings::init()?,
        ConfigCommands::Show => {
            let settings = Settings::load()?;
            println!("{}", settings.display("demo"));
        }
        ConfigCommands::Set { key, value } => {
            let mut settings = Settings::load()?;
            settings.set("demo", &key, &value)?;
            settings.save()?;
            println!("已设置 demo.{} = {}", key, value);
        }
    }
    Ok(())
}

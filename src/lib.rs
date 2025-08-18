#![deny(clippy::unwrap_used)]

use crate::config::ConfigArgs;

pub mod aws;
pub mod config;
pub mod tunnelblick;

pub fn init(name: &str, config_args: &ConfigArgs) -> anyhow::Result<()> {
    config::load_env_files(name, config_args)?;

    let env_name = format!("{}_LOG", name.replace('-', "_").to_uppercase());
    let filter = config::get_opt(&env_name)?.unwrap_or_else(|| "warn".to_string());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    Ok(())
}

pub const XKCD_EXPECT_MSG: &str = "\n\n==============================================================================\nIf you're seeing this, the code is in what I thought was an unreachable state.\nI could give you advice for what to do. But honestly, why should you trust me?\nI clearly screwed this up. I'm writing a message that should never appear, yet\nI know it will probably appear someday.\n\nOn a deep level, I know I'm not up to this task. I'm so sorry.\n\n ~ https://xkcd.com/2200/\n==============================================================================\n\n";

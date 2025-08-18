use anyhow::Context;
use std::env::VarError;
use std::path::PathBuf;

#[derive(clap::Args, Debug, Clone)]
pub struct ConfigArgs {
    #[clap(short, long, global = true)]
    /// The profile to load
    pub profile: Option<String>,
}

pub fn load_env_files(app_name: &str, args: &ConfigArgs) -> anyhow::Result<()> {
    let config_dir = {
        let xdg_config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(|config_path| PathBuf::from(config_path).join("dcli"))
            .ok()
            .filter(|path| std::fs::exists(path).is_ok_and(|exists| exists));

        let home_config_dir = std::env::var("HOME")
            .map(|home_dir| PathBuf::from(home_dir).join(".dcli"))
            .ok()
            .filter(|path| std::fs::exists(path).is_ok_and(|exists| exists));

        match (xdg_config_dir, home_config_dir) {
            (Some(xdg_config_dir), Some(home_config_dir)) => {
                anyhow::bail!(
                    "found configurations in both {} and {}. Please merge configurations in one location",
                    xdg_config_dir.display(),
                    home_config_dir.display()
                )
            }
            (Some(xdg_config_dir), None) => {
                tracing::info!("Using XDG_CONFIG_HOME={}", xdg_config_dir.display());
                xdg_config_dir
            }
            (None, Some(home_config_dir)) => {
                tracing::info!("Using HOME={}", home_config_dir.display());
                home_config_dir
            }
            (None, None) => {
                anyhow::bail!(
                    "Unable to find configuration in either $HOME/.dcli or $XDG_CONFIG_HOME/dcli. Please create one of them and try again."
                );
            }
        }
    };

    // <profile>.env is mandatory if selected. otherwise try to load fallback profile
    if let Some(profile) = args.profile.as_deref() {
        load_file(config_dir.join(format!("{profile}.env")))
            .context("Unable to load configuration file for profile")?;
    } else {
        load_file_opt(config_dir.join("default.env"))?;
    }

    // per-tool config (optional)
    load_file_opt(config_dir.join(format!("{app_name}.env")))?;

    // global config (optional)
    load_file_opt(config_dir.join("global.env"))?;

    Ok(())
}

fn load_file(path: PathBuf) -> anyhow::Result<()> {
    tracing::debug!(file = %path.display(), "loading mandatory file");
    dotenvy::from_path(path).context("File is required")
}

fn load_file_opt(path: PathBuf) -> anyhow::Result<()> {
    tracing::debug!(file = %path.display(), "loading optional file");
    if let Err(e) = dotenvy::from_path(&path)
        && !e.not_found()
    {
        tracing::debug!(file = %path.display(), "file does not exist, ignoring");
        Err(e)?;
    }

    Ok(())
}

pub fn get(name: &str) -> anyhow::Result<String> {
    get_opt(name)?.ok_or_else(|| is_mandatory_err(name))
}

pub fn get_opt(name: &str) -> anyhow::Result<Option<String>> {
    let plain = read_env_var(name)?;

    // TODO: Add encryption support

    Ok(plain)
}

pub fn get_bool(name: &str) -> anyhow::Result<bool> {
    get_bool_opt(name)?.ok_or_else(|| is_mandatory_err(name))
}

pub fn get_bool_opt(name: &str) -> anyhow::Result<Option<bool>> {
    let Some(value) = get_opt(name)? else {
        return Ok(None);
    };
    match value.as_str() {
        "yes" | "true" | "1" | "on" | "enable" | "enabled" => Ok(Some(true)),
        "no" | "false" | "0" | "off" | "disable" | "disabled" => Ok(Some(false)),
        _ => Err(anyhow::anyhow!(
            "Invalid boolean value for {name}: `{value}`"
        ))?,
    }
}

pub fn get_json<T: serde::de::DeserializeOwned>(name: &str) -> anyhow::Result<T> {
    get_json_opt(name)?.ok_or_else(|| is_mandatory_err(name))
}

pub fn get_json_opt<T: serde::de::DeserializeOwned>(name: &str) -> anyhow::Result<Option<T>> {
    let Some(json) = get_opt(name)? else {
        return Ok(None);
    };

    serde_json::from_str(&json).context(format!("Invalid JSON for {name}: `{json}`"))
}

fn read_env_var(name: &str) -> anyhow::Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value).filter(|value| !value.is_empty())),
        Err(VarError::NotPresent) => Ok(None),
        Err(e) => Err(e).context(format!(
            "Environment variable value can not be parsed as UTF-8 ({name})"
        )),
    }
}

fn is_mandatory_err(name: &str) -> anyhow::Error {
    anyhow::anyhow!("{name} is not set, but it is required")
}

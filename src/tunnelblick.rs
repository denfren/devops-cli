use osascript::{Error, JavaScript};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const TUNNELBLICK_CONFIG: &str = "tunnelblick";

type Result<T> = std::result::Result<T, TunnelblickError>;

pub fn get_status() -> Result<Vec<Vpn>> {
    let script = JavaScript::new(
        r##"
var tblk = Application('Tunnelblick')
var configs = []

var cfg = tblk.configurations().length
for(let i = 0;i<cfg;i++) {
  let c = tblk.configurations[i];
  configs.push({name: c.name(),  state: c.state()})
}
return configs
    "##,
    );

    Ok(script.execute()?)
}

pub fn connect(vpn_name: &str) -> Result<ChangeResult> {
    let result = JavaScript::new(
        r##"var changed = Application('Tunnelblick').connect($params);return {changed: changed};"##,
    )
    .execute_with_params(vpn_name)?;

    Ok(result)
}

pub fn disconnect(vpn_name: &str) -> Result<ChangeResult> {
    let result =
        JavaScript::new(r##"var changed = Application('Tunnelblick').disconnect($params);return {changed: changed};"##)
            .execute_with_params(vpn_name)?;

    Ok(result)
}

pub fn disconnect_all() -> Result<DisconnectResult> {
    let result = JavaScript::new(
        r##"var count = Application("Tunnelblick").disconnectAll();return {count: count};"##,
    )
    .execute()?;

    Ok(result)
}

#[derive(Deserialize)]
pub struct ChangeResult {
    pub changed: bool,
}

#[derive(Deserialize)]
pub struct DisconnectResult {
    pub count: i32,
}

#[derive(Deserialize, Serialize, Eq, PartialEq)]
pub struct Vpn {
    pub name: String,
    pub state: State,
}

#[derive(Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    Connected,
    Auth,
    GetConfig,
    Exiting,
    Disconnecting,
    #[serde(other)]
    Unknown,
}

#[derive(Error, Debug)]
pub enum TunnelblickError {
    #[error("Unable to parse response from tunnelblick")]
    ScriptResponseError(#[source] osascript::Error),

    #[error("Unable to run osascript to control tunnelblick")]
    ScriptExecutionError(#[source] osascript::Error),

    #[error("The script to control tunnelblick is not compatible with your version")]
    ScriptNotCompatible(#[source] osascript::Error),
}

impl From<osascript::Error> for TunnelblickError {
    fn from(e: Error) -> Self {
        match e {
            Error::Io(_) => TunnelblickError::ScriptExecutionError(e),
            Error::Json(_) => TunnelblickError::ScriptResponseError(e),
            Error::Script(_) => TunnelblickError::ScriptNotCompatible(e),
        }
    }
}

pub async fn wait_for_state<F>(
    wait: std::time::Duration,
    retries: u32,
    f: F,
) -> anyhow::Result<bool>
where
    F: Fn(Vec<Vpn>) -> anyhow::Result<bool>,
{
    for _ in 1..=retries {
        let status = get_status()?;
        match f(status) {
            Ok(false) => tokio::time::sleep(wait).await,
            failure_or_success => return failure_or_success,
        }
    }

    Ok(false)
}

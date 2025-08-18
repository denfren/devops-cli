use devops_cli as cli;
use devops_cli::config;

pub const DVPN_TUNNELBLICK_CONNECTION: &str = "DVPN_TUNNELBLICK_CONNECTION";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = clap::Parser::parse();
    cli::init(env!("CARGO_BIN_NAME"), &cli.config)?;

    if cli.disconnect {
        tracing::info!("Disconnecting all VPN connections");
        cli::tunnelblick::disconnect_all()?;
        return Ok(());
    }

    let connection = config::get(DVPN_TUNNELBLICK_CONNECTION)?;

    let status = cli::tunnelblick::get_status()?;

    let wants_connect = status
        .iter()
        .any(|s| s.name == connection && s.state != cli::tunnelblick::State::Connected);
    anyhow::ensure!(
        wants_connect,
        "{connection} is already connected. Nothing to do.",
    );

    let wants_disconnect = status
        .iter()
        .any(|s| s.state != cli::tunnelblick::State::Exiting && s.name != connection);

    if wants_disconnect {
        anyhow::ensure!(
            dialoguer::Confirm::new()
                .with_prompt("Disconnect other VPN connections?")
                .interact()?,
            "user said no."
        );
        cli::tunnelblick::disconnect_all().map_err(|e| anyhow::anyhow!(e))?;

        tracing::info!("waiting for all connections to exit");
        cli::tunnelblick::wait_for_state(std::time::Duration::from_secs(1), 60, |v| {
            anyhow::ensure!(!v.is_empty(), "No connections to wait for to disconnect");
            Ok(v.iter()
                .all(|c| c.state == cli::tunnelblick::State::Exiting))
        })
        .await?;
    }

    let connecting = cli::tunnelblick::connect(&connection)
        .map_err(|e| anyhow::anyhow!(e))?
        .changed;

    if !connecting {
        anyhow::bail!("`{}` is already connected. Nothing to do.", connection);
    }

    eprint!("Waiting for connection (Duo?)...");

    cli::tunnelblick::wait_for_state(std::time::Duration::from_secs(1), 300, |c| {
        let item = c.iter().find(|s| s.name == connection);
        if let Some(v) = item {
            eprint!(".");
            Ok(v.state == cli::tunnelblick::State::Connected)
        } else {
            anyhow::bail!("No connection named {}", connection);
        }
    })
    .await?;

    eprintln!("connected!");

    Ok(())
}

#[derive(clap::Parser, Debug)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(flatten)]
    pub config: config::ConfigArgs,

    #[arg(short, long)]
    /// Disconnect
    pub disconnect: bool,
}

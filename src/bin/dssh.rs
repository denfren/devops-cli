use cli::config;
use devops_cli as cli;

const DSSH_TUNNELBLICK_CONNECTION: &str = "DSSH_TUNNELBLICK_CONNECTION";
const DSSH_TEMPLATE_DISPLAY: &str = "DSSH_TEMPLATE_DISPLAY";
const DSSH_TEMPLATE_COMMAND: &str = "DSSH_TEMPLATE_COMMAND";
const DSSH_TEMPLATE_MULTICOMMAND: &str = "DSSH_TEMPLATE_MULTICOMMAND";

const DEFAULT_COMMAND_TEMPLATE: &[&str] = &["ssh", "{{ private_ip }}"];
const DEFAULT_DISPLAY_TEMPLATE: &str =
    "{{ tags.Name|default('<unnamed instance>') }} ({{ id }} ({{ state }})";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command_separator = std::env::args_os()
        .position(|sep| sep == "::")
        .unwrap_or(std::env::args_os().len());

    let opts: Cli = clap::Parser::parse_from(std::env::args_os().take(command_separator));
    cli::init(env!("CARGO_BIN_NAME"), &opts.config)?;

    let config_display_template = if let Some(template) = opts.template {
        template
    } else if let Some(template) = config::get_opt(DSSH_TEMPLATE_DISPLAY)? {
        template
    } else {
        DEFAULT_DISPLAY_TEMPLATE.to_string()
    };

    let config_cmd = config::get_json_opt::<Vec<String>>(DSSH_TEMPLATE_COMMAND)?;
    let config_multicmd = config::get_json_opt::<Vec<String>>(DSSH_TEMPLATE_MULTICOMMAND)?;

    // check VPN connectivity
    if let Some(tunnelblick_connection) = config::get_opt(DSSH_TUNNELBLICK_CONNECTION)? {
        let vpn_status = cli::tunnelblick::get_status()?;
        let wanted = vpn_status
            .iter()
            .find(|vpn| vpn.name == tunnelblick_connection)
            .ok_or_else(|| {
                anyhow::anyhow!("Tunnelblick connection {tunnelblick_connection} not found")
            })?;

        anyhow::ensure!(
            wanted.state == cli::tunnelblick::State::Connected,
            "VPN {} is not connected",
            tunnelblick_connection
        );
    }

    let instances = {
        let aws_client = cli::aws::AwsClient::new().await;

        let mut instances = cli::aws::list_instances(&opts.ec2_select, &aws_client).await?;
        anyhow::ensure!(!instances.is_empty(), "search returned no results");

        instances.sort_by_key(|image| std::cmp::Reverse(image.launch_time));
        instances
    };

    let jinja_env = minijinja::Environment::new();

    let display_list = instances
        .iter()
        .map(|instance| {
            jinja_env
                .render_str(&config_display_template, instance)
                .map_err(Into::into)
        })
        .collect::<anyhow::Result<Vec<String>>>()?;

    let selected_instances = {
        if instances.len() == 1 {
            instances.iter().collect::<Vec<_>>()
        } else if !opts.multi {
            dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("select instances")
                .items(&display_list)
                .clear(true)
                .interact_opt()?
                .iter()
                .map(|i| &instances[*i])
                .collect::<Vec<_>>()
        } else {
            dialoguer::MultiSelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("select instances")
                .clear(true)
                .report(false)
                .items(&display_list)
                .defaults(&vec![true; display_list.len()])
                .interact_opt()?
                .iter()
                .flatten()
                .map(|i| &instances[*i])
                .collect::<Vec<_>>()
        }
    };

    anyhow::ensure!(!selected_instances.is_empty(), "no instances selected");

    let cli_cmd = Some(
        std::env::args()
            .skip(command_separator + 1)
            .collect::<Vec<String>>(),
    )
    .filter(|list| !list.is_empty());
    let command_template = cli_cmd
        .or(if selected_instances.len() == 1 {
            config_cmd
        } else {
            config_multicmd
        })
        .unwrap_or_else(|| {
            DEFAULT_COMMAND_TEMPLATE
                .iter()
                .map(ToString::to_string)
                .collect()
        });

    let commands = selected_instances
        .iter()
        .map(|instance| {
            command_template
                .iter()
                .map(|cmd| Ok(jinja_env.render_str(cmd, instance)?))
                .collect::<anyhow::Result<Vec<String>>>()
        })
        .collect::<anyhow::Result<Vec<Vec<String>>>>()?;

    if opts.dry_run {
        for command in commands {
            println!("{}", command.join(" "));
        }
        return Ok(());
    }

    use std::process::Command;
    if commands.len() == 1 {
        use std::os::unix::process::CommandExt;
        Err(Command::new(&commands[0][0]).args(&commands[0][1..]).exec())?;
    } else {
        for command in commands {
            Command::new(&command[0]).args(&command[1..]).spawn()?;
        }
    }

    Ok(())
}

#[derive(clap::Parser, Debug)]
#[command(author, version)]
/// Connect to AWS EC2 instances via SSH
pub struct Cli {
    #[command(flatten)]
    pub config: config::ConfigArgs,

    #[command(flatten)]
    pub ec2_select: cli::aws::Ec2SelectArgs,

    #[arg(short = 'T', long)]
    /// Template to use to display instances in the selection dialog.
    pub template: Option<String>,

    #[arg(short, long)]
    /// Allow multiple instances to be selected.
    pub multi: bool,

    #[arg(short = 'n', long)]
    /// Do not execute the command, just print it.
    pub dry_run: bool,
}

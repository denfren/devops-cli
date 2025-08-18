use aws_sdk_ec2::types::{Instance, InstanceState, InstanceStateChange};
use aws_smithy_types_convert::date_time::DateTimeExt;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::ops::BitAnd;
use thiserror::Error;

pub struct AwsClient(aws_types::SdkConfig);

impl AwsClient {
    pub async fn new() -> AwsClient {
        // We can't set unlimited, so we set a sufficiently high value (12h)
        let long_enough = chrono::Duration::hours(12)
            .to_std()
            .expect(crate::XKCD_EXPECT_MSG);

        let cache = aws_config::identity::LazyCacheBuilder::default()
            .load_timeout(long_enough)
            .build();

        let aws_config = aws_config::ConfigLoader::default()
            .behavior_version(aws_config::BehaviorVersion::v2025_01_17())
            .identity_cache(cache);

        AwsClient(aws_config.load().await)
    }

    pub async fn query_instances(
        &self,
        filters: Vec<QueryFilter>,
    ) -> Result<Vec<Ec2Instance>, AwsClientError> {
        let client = aws_sdk_ec2::client::Client::new(&self.0);
        let mut operation = client.describe_instances();

        for filter in filters {
            operation = operation.filters(filter.into());
        }

        let result = operation.max_results(1000).send().await?;

        let reservations: Vec<aws_sdk_ec2::types::Reservation> = result
            .reservations
            .expect("AWS provided instance data did not include reservations, which is expected");
        let instances: Vec<_> = reservations
            .into_iter()
            .flat_map(|r| r.instances.unwrap_or_default())
            .flat_map(std::convert::TryInto::try_into)
            .collect();

        Ok(instances)
    }
}

#[derive(Deserialize)]
pub struct AwsConfig {
    /// The profile to use
    pub profile: Option<String>,
}

#[derive(Error, Debug)]
pub enum AwsClientError {
    #[error(transparent)]
    SdkError(Box<dyn std::error::Error + Send + Sync>),
}

impl<E> From<aws_sdk_ec2::error::SdkError<E>> for AwsClientError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(e: aws_sdk_ec2::error::SdkError<E>) -> Self {
        AwsClientError::SdkError(Box::new(e))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ec2Instance {
    pub id: String,
    pub state: Ec2InstanceState,
    pub availability_zone: String,
    pub private_ip: String,
    pub private_dns: Option<String>,
    pub public_ip: Option<String>,
    pub public_dns: Option<String>,
    pub launch_time: chrono::DateTime<chrono::Utc>,
    pub tags: indexmap::IndexMap<String, String>,
}

impl Ec2Instance {
    #[must_use]
    pub fn to_short_string(&self) -> String {
        let name = self.tags.get("Name").map_or("", |s| s.as_str());
        format!("{} ({}, {:?})", name, self.id, self.state)
    }
}

impl TryFrom<Instance> for Ec2Instance {
    type Error = ParseError;

    fn try_from(value: Instance) -> Result<Self, Self::Error> {
        let mut tags: indexmap::IndexMap<_, _> = value
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|v| (v.key.unwrap_or_default(), v.value.unwrap_or_default()))
            .collect();
        tags.sort_keys();

        let instance = Ec2Instance {
            id: value.instance_id.ok_or(ParseError("instance_id"))?,
            state: value.state.ok_or(ParseError("state"))?.into(),
            availability_zone: value
                .placement
                .ok_or(ParseError("placement"))?
                .availability_zone
                .ok_or(ParseError("placement.availability_zone"))?,
            private_ip: value.private_ip_address.ok_or(ParseError("ip_private"))?,
            private_dns: value.private_dns_name.filter(|s| !s.is_empty()),
            public_ip: value.public_ip_address.filter(|s| !s.is_empty()),
            public_dns: value.public_dns_name.filter(|s| !s.is_empty()),
            launch_time: value
                .launch_time
                .and_then(|dt| dt.to_chrono_utc().ok())
                .ok_or(ParseError("launch_time"))?,
            tags,
        };
        Ok(instance)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub enum Ec2InstanceState {
    #[default]
    Unknown,
    Pending,
    Running,
    ShuttingDown,
    Terminated,
    Stopping,
    Stopped,
}

impl From<InstanceState> for Ec2InstanceState {
    // Lower bits
    //  0 : pending
    // 16 : running
    // 32 : shutting-down
    // 48 : terminated
    // 64 : stopping
    // 80 : stopped
    fn from(s: InstanceState) -> Self {
        match s.code.unwrap_or(0).bitand(0xFF) {
            0 => Ec2InstanceState::Pending,
            16 => Ec2InstanceState::Running,
            32 => Ec2InstanceState::ShuttingDown,
            48 => Ec2InstanceState::Terminated,
            64 => Ec2InstanceState::Stopping,
            80 => Ec2InstanceState::Stopped,
            _ => Ec2InstanceState::Unknown,
        }
    }
}

impl Display for Ec2InstanceState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Ec2InstanceState::Pending => "pending",
            Ec2InstanceState::Running => "running",
            Ec2InstanceState::ShuttingDown => "shutting down",
            Ec2InstanceState::Terminated => "terminated",
            Ec2InstanceState::Stopping => "stopping",
            Ec2InstanceState::Stopped => "stopped",
            Ec2InstanceState::Unknown => "unknown",
        };
        f.write_str(name)
    }
}

#[derive(Serialize, Debug)]
pub struct Ec2InstanceStateChange {
    pub instance_id: String,
    pub previous_state: Ec2InstanceState,
    pub current_state: Ec2InstanceState,
}

impl From<InstanceStateChange> for Ec2InstanceStateChange {
    fn from(i: InstanceStateChange) -> Self {
        Self {
            instance_id: i.instance_id.unwrap_or_default(),
            previous_state: i
                .previous_state
                .map(std::convert::Into::into)
                .unwrap_or_default(),
            current_state: i
                .current_state
                .map(std::convert::Into::into)
                .unwrap_or_default(),
        }
    }
}

#[derive(Error, Debug)]
#[error("Missing field {0}")]
pub struct ParseError(&'static str);

#[derive(clap::Args, Debug)]
pub struct Ec2SelectArgs {
    /// Raw filters passed to AWS API (`--filter key=value[,value2...]`)
    ///
    /// For possible values see the 'Filter.N' section of DescribeInstances:
    /// https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_DescribeInstances.html
    #[clap(short, long, verbatim_doc_comment)]
    pub filter: Option<Vec<String>>,

    /// Only 'regular' doc-proc servers
    ///
    /// This is a shortcut for `--filter tag:AV_Scan=false --filter tag:StepfileProcessor=false`
    #[clap(long)]
    pub docproc: bool,

    /// Only av-scan doc-proc servers
    ///
    /// This is a shortcut for `--filter tag:AV_Scan=true`
    #[clap(long)]
    pub avscan: bool,

    /// Exclude av-scan doc-proc servers
    ///
    /// This is a shortcut for `--filter tag:AV_Scan=false`
    #[clap(long, conflicts_with = "avscan")]
    pub no_avscan: bool,

    /// Only stepfile-processor doc-proc servers
    ///
    /// This is a shortcut for `--filter tag:StepfileProcessor=true`
    #[clap(long)]
    pub stepfile: bool,

    /// Exclude stepfile-processor doc-proc servers
    ///
    /// This is a shortcut for `--filter tag:StepfileProcessor=false`
    #[clap(long, conflicts_with = "stepfile")]
    pub no_stepfile: bool,

    /// Filter instances
    ///
    /// - start with `i-` to filter (starts_with) match on aws instance id
    /// - numbers will only match on the end of the name, intended for cluster-id filtering
    /// - anything else will be matched (contains) on the tag `Name`
    #[clap(verbatim_doc_comment)]
    pub query: Vec<String>,
}

impl Ec2SelectArgs {
    pub fn filter_with_extra_flags(&self) -> Vec<String> {
        let mut filters = self.filter.as_ref().cloned().unwrap_or_default();
        if self.docproc {
            filters.push("tag:AV_Scan=false".to_string());
            filters.push("tag:StepfileProcessor=false".to_string());
        }
        if self.avscan {
            filters.push("tag:AV_Scan=true".to_string());
        }
        if self.no_avscan {
            filters.push("tag:AV_Scan=false".to_string());
        }
        if self.stepfile {
            filters.push("tag:StepfileProcessor=true".to_string());
        }
        if self.no_stepfile {
            filters.push("tag:StepfileProcessor=false".to_string());
        }

        filters
    }

    pub fn has_no_filters(&self) -> bool {
        self.query.is_empty()
            && self.filter.as_ref().map(|f| f.is_empty()).unwrap_or(true)
            && !self.avscan
            && !self.no_avscan
            && !self.stepfile
            && !self.docproc
            && !self.no_stepfile
    }
}

pub async fn list_instances(
    opts: &Ec2SelectArgs,
    client: &AwsClient,
) -> anyhow::Result<Vec<Ec2Instance>> {
    let filters: Result<Vec<_>, ()> = opts
        .filter_with_extra_flags()
        .iter()
        .map(|f| f.parse())
        .collect();
    let filters = filters.map_err(|_| anyhow::anyhow!("Unable to parse filters"))?;

    let instances = client.query_instances(filters).await?;
    let user_query = Ec2InstanceFilter::new(opts.query.clone());
    let mut instances: Vec<_> = instances
        .into_iter()
        .filter(|i| user_query.filter(i))
        .collect();

    instances.sort_by_key(|i| std::cmp::Reverse(i.launch_time));

    Ok(instances)
}

#[derive(Debug, Eq, PartialEq)]
enum Ec2InstanceFilterKind {
    /// AWS instance id in the format `i-abcdefghijk`
    AwsInstanceId(String),
    /// A numeric identifier, used in clusters (1, 2, 3, ...)
    InstanceId(u8),
    /// Free form text to match
    Text(Vec<String>),
}

impl std::str::FromStr for Ec2InstanceFilterKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(());
        }

        if s.starts_with("i-") {
            return Ok(Ec2InstanceFilterKind::AwsInstanceId(s.to_string()));
        }

        if let Ok(i) = s.parse::<u8>() {
            return Ok(Ec2InstanceFilterKind::InstanceId(i));
        }

        Ok(Ec2InstanceFilterKind::Text(
            s.split(',').map(String::from).collect(),
        ))
    }
}

struct Ec2InstanceFilter(Vec<Ec2InstanceFilterKind>);

impl Ec2InstanceFilter {
    pub fn new(f: Vec<String>) -> Self {
        let filters: Vec<Ec2InstanceFilterKind> = f.into_iter().flat_map(|s| s.parse()).collect();

        Self(filters)
    }

    pub fn filter(&self, i: &Ec2Instance) -> bool {
        self.0.iter().all(|filter| match filter {
            Ec2InstanceFilterKind::AwsInstanceId(id) => i.id.starts_with(id),
            Ec2InstanceFilterKind::InstanceId(id) => i
                .tags
                .get("Name")
                .map(|name| name.ends_with(&format!("{id}")))
                .unwrap_or_default(),
            Ec2InstanceFilterKind::Text(query) => i
                .tags
                .get("Name")
                .map(|name| {
                    query.iter().any(|q| {
                        if let Some(q) = q.strip_prefix('_') {
                            !name.contains(q)
                        } else {
                            name.contains(q)
                        }
                    })
                })
                .unwrap_or_default(),
        })
    }
}

#[derive(new, Debug)]
pub struct QueryFilter {
    pub key: String,
    pub values: Vec<String>,
}

impl std::str::FromStr for QueryFilter {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (key, values) = value.split_once('=').ok_or(())?;

        let key = key.to_owned();
        let values: Vec<String> = values.split(',').map(ToOwned::to_owned).collect();

        Ok(QueryFilter { key, values })
    }
}

impl From<QueryFilter> for aws_sdk_ec2::types::Filter {
    fn from(f: QueryFilter) -> Self {
        aws_sdk_ec2::types::Filter::builder()
            .name(f.key)
            .set_values(Some(f.values).filter(|v| !v.is_empty()))
            .build()
    }
}

impl From<QueryFilter> for aws_sdk_autoscaling::types::Filter {
    fn from(f: QueryFilter) -> Self {
        aws_sdk_autoscaling::types::Filter::builder()
            .name(f.key)
            .set_values(Some(f.values).filter(|v| !v.is_empty()))
            .build()
    }
}

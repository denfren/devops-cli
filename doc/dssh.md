# dssh

`dssh` spawns ssh sessions by querying for EC2 instances. It can also be used to invoke arbitrary commands using
information from the instances.

## Basic usage

`dssh [--profile <profile>] [<query>...]` query EC2 instances and spawn ssh session

- `query` limits the instance selection. When only one instance matches the command executes immediately

Query words can have three forms

* `i-...` will only be matched against the aws instance id
* numeric values will only match on the end of the name - intended for clustered instances
* anything else will text match on the instance name

Given three instances

* `www_server-fra-i-122222-01`
* `sql_db_server-dus-i-133333-01`
* `sql_db_server-dus-i-144444-02`

Then running...

* `dssh ww` immediately connects to the www_server (single result)
* `dssh db` shows a list of the db servers to pick one to connect to (multiple results)
* `dssh db 2` finds the instance whose name contains both `db` and the cluster instance is `2`, despite the first
  instance having a literal `2` in its name (single result)
* `dssh i-13` connects to sql db 01, because its instance id starts with `i-13`

## Configuration

- `DSSH_LOG` (default: `"warn"`): configure log level
- `DSSH_TEMPLATE_DISPLAY` (string): template to use to display instances in the selection dialog. Supports Jinja2
  templating with instance data
- `DSSH_TEMPLATE_COMMAND` (json, a list of strings): command template to execute for single instance selections. strings
- `DSSH_TEMPLATE_MULTICOMMAND`  (json, a list of strings): command template to execute when multiple instances are
  selected. array of strings
- `DSSH_TUNNELBLICK_CONNECTION`: A name of a tunnelblick connection to check if it is connected.

## Templates

Fields available in the template context (instance details)

- id
- state
- availability_zone
- private_ip
- private_dns
- public_ip
- public_dns
- launch_time
- tags.<tag-name>

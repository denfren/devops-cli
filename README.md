# DevOps CLI

A suite of cli tools for DevOps

See [config](#Config) for an introduction to configuration and [example](#Example) for typical configuration.

## dssh

`dssh` spawns ssh sessions by querying for EC2 instances. It can also be used to invoke arbitrary commands using
information from the instances.

### Basic usage

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

### dssh configuration

- AWS specific environment variables, like `AWS_PROFILE`
- `DSSH_LOG` (default: `"warn"`): configure log level
- `DSSH_TEMPLATE_DISPLAY` (string): template to use to display instances in the selection dialog. Supports Jinja2
  templating with instance data
- `DSSH_TEMPLATE_COMMAND` (json, a list of strings): command template to execute for single instance selections.
  Defaults to `"ssh", "{{ private_ip }}"`, example: `["ssh", "admin@{{ private_dns }}", "-p", "2222"]`
- `DSSH_TEMPLATE_MULTICOMMAND`  (json, a list of strings): command template to execute when multiple instances are
  selected. Defaults to the value of the `DSSH_TEMPLATE_COMMAND`
- `DSSH_TUNNELBLICK_CONNECTION`: A name of a tunnelblick connection to check if it is connected.

### Templates

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

## dvpn

`dvpn`, `dvpn --profile <profile>` connects to the VPN.
`dvpn -d`, `dvpn --disconnect` disconnects all VPNs.

### dvpn configuration

- `DVPN_LOG` (default: `"warn"`): configure log level
- `DVPN_TUNNELBLICK_CONNECTION`: the name of the connection in tunnelblick

## Config

The cli commands are configured using environment variables. To simplify configuration, values can be specified in `env`
files at the following locations:

1. `$XDG_CONFIG_HOME/dcli/*.env`
2. `$HOME/.dcli/*.env`

When running commands, a "profile" can be specified using `-p/--profile <profile-name>`. This affects which env files
are loaded. In the order listed below, values are loaded:

1. Values that are already set in the environment take precedence
2. if a profile is specified on the command line, **`<profile>.env`** is loaded. Otherwise **`default.env`** is loaded.
3. then **`<tool-name>.env`** is loaded and can be used to provide tool-specific configuration used across all profiles.
4. finally, **`global.env`** can be used to configure across all profiles and all tools.

### Value types

Values that are the empty are considered to be unset. This allows overriding values to remove a setting.

- **string** values are used as is
- **boolean** can be specified using the following values:
    - `"yes"`, `"true"`, `"1"`, `"on"`, `"enable"`, `"enabled"`
    - `"no"`, `"false"`, `"0"`, `"off"`, `"disable"`, `"disabled"`
- **json** values are used for more complex configuration, such as lists or maps

### Example

**Typical configuration**

`$HOME/.dcli/staging.env` and `$HOME/.dcli/prod.env`

```shell
AWS_PROFILE=my-role-name
DVPN_TUNNELBLICK_CONFIG=my-tunnelblick-config-name
```

**Use staging by default**

Simply symlink default.env to staging.env:

```shell
ln -s $HOME/.dcli/staging.env $HOME/.dcli/default.env
```

**Let dssh check VPN configured for dvpn**

use `global.env` to let DSSH use DVPN's value by default:

```shell
DSSH_TUNNELBLICK_CONNECTION=${DVPN_TUNNELBLICK_CONNECTION}
```

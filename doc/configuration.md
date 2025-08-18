# Configuration

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

## Value types

Values that are the empty are considered to be unset. This allows overriding values to remove a setting.

- **string** values are used as is
- **boolean** can be specified using the following values:
    - `"yes"`, `"true"`, `"1"`, `"on"`, `"enable"`, `"enabled"`
    - `"no"`, `"false"`, `"0"`, `"off"`, `"disable"`, `"disabled"`
- **json** values are used for more complex configuration, such as lists or maps

## Tips

Set `DSSH_TUNNELBLICK_CONNECTION=${DVPN_TUNNELBLICK_CONNECTION}` in `global.env` to enable `dssh` to check the vpn
connection, if one is configured for `dvpn`. This can be overridden by setting it to the empty string in a profile.
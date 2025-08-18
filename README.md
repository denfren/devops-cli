# DevOps CLI

A suite of cli tools for DevOps

see [doc/configuration.md](doc/configuration.md) for guidance on how to configure the tools.

## dssh

see [doc/dssh.md](./doc/dssh.md)

- `dssh` lists all EC2 instances, upon selection, connect to it using ssh
- `dssh loadbalancer` connects to the EC2 instance with the name loadbalancer

Advanced usages:

- connecting to multiple instances at once, using tmux
- run local commands, templated from selected instance information

## dvpn

see [doc/dvpn.md](./doc/dvpn.md)

- `dvpn` connect to the VPN (Tunnelblick)
 
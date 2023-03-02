# flopsy
A general purpose failover proxy.


## What does that mean?
Flopsy is a protocol-agnostic proxy server with support for failover across multiple backends.
On startup, one backend is selected as the _primary_ backend. Once a backend is selected,
flopsy starts accepting connections, opening a new connection to the primary backend for each
incoming connection and relaying data between the two connections.

If the primary backend goes down, flopsy will attempt to select a new primary backend.
The first primary to (a) accept connections and (b) pass an optional, user-specified host check
is selected as the new primary.
No new connections are accepted until a new primary has been selected _and_ an optional list of
failover _triggers_ have been executed.


## Checks and triggers
Checks and triggers are user-specified scripts which are executed upon successful connection to
a primary candidate and completion of the primary selection process respectively.
Both script types receive the connection string of the host in question as their only argument,
and must be executable.

### Check scripts
Checks are used to determine whether a given host is usable as a primary or not.
A check script must use its exit status code to signal whether the given host is usable or not,
exiting with a 0 status code if the host is usable, or with any non-zero status code if it is not.

### Trigger scripts
Trigger scripts are used to perform some action, such as preparing the new primary or notifying
some other service, when a new failover has been selected.
Flopsy will not accept any new connections until all trigger scripts have run to completion.
A trigger script exiting with a non-zero status code will be logged as a warning,
but will not otherwise impact execution.


## Basic use
Relaying incoming connections on port 8080 to whichever of `host1` and `host2` is currently
listening on port 8080:
```
flopsy -p 8080 -H host1:8080,host2:8080
```

As above, but require that hosts also respond to ping before being selected as primary:
```
flopsy -p 8080 -H host1:8080,host2:8080 -c ./ping.sh
```
Where `./ping.sh` contains:
```bash
#!/bin/sh
exec ping -c1 $(echo $1 | sed -e 's/:.*$//')
```

For more information, see `flopsy --help`.


## Running in Docker
The flopsy docker image can be configured using the following environment variables:
- `PORT`: port on which to listen for connections. Equivalent to `--port`.
- `HOSTS`: comma-separated list from which to select a primary backend. Equivalent to `--hosts`
- `MAX_BACKOFF`: Maximum time to wait between attempts to reselect a new primary after the first one goes down. Equivalent to `--max-backoff`.

Additionally, the following paths are checked for scripts, which may be mounted or otherwise injected into the container:
- `/etc/flopsy/check-host.sh`: used as check script, if present.
- `/etc/flopsy/triggers.d`: any files in this directory will be executed as trigger scripts.


## FAQ
- Help, the build fails on non-Linux OSes!
  - Flopsy by default depends on the `splice` system call, which is only available on Linux.
    To build on other platforms, pass the `--no-default-features` flag to `cargo install`,
    to use a portable, but significantly slower, implementation instead.
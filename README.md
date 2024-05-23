# neptune-explorer

A web-based block explorer for the [Neptune blockchain](https://neptune.cash).  neptune-explorer provides a basic HTML view and a REST RPC API.

As of 2024-05-22 this code is running at https://explorer.neptune.cash.

Some [design notes](./doc/design_notes.md) are available.

## Installing

### Compile from Source -- Linux Debian/Ubuntu

```
git clone https://github.com/Neptune-Crypto/neptune-explorer.git
cd neptune-explorer
cargo install --locked --path .
```

### Windows, Mac

not tested or supported.   Please let us know if you get it work.  patches accepted.

## Running

1. install [neptune-core](https://github.com/Neptune-Crypto/neptune-core) and start it, or otherwise find a running neptune-core instance.
2. start neptune-explorer

```
nohup neptune-explorer 2>&1 > /path/to/logs/neptune-explorer.log &
```

Notes:
* The block-explorer automatically uses the same network (mainnet, testnet, etc) as the neptune-core instance it is connected to, and the network is displayed in the web interface.
* if neptune-core RPC server is running on a non-standard port, you can provide it with the `--port` flag.
* Site name can be specified with the `--site-name` flag.


## Connecting via Browser

Just navigate to http://localhost:3000/

(note: the port is not yet configurable.)


## SSL/TLS, Nginx, etc.

If hosting for public use, it is suggested to use nginx or similar in reverse-proxy mode to connect to `http://localhost:3000`.  Nginx can then handle SSL/TLS certs and connections, as neptune-explorer has no built-in support for that.


## Logging

All logging is output to standard out.

The log level can be set through the environment variable `RUST_LOG`. Valid values are: `trace`, `debug`, `info`, `warn`, and `error`. The default value is `info`. E.g.: `RUST_LOG=trace cargo run`.

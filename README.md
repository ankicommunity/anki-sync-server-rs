# anki-sync-server-rs

[![](https://img.shields.io/github/v/release/ankicommunity/anki-sync-server-rs)](https://github.com/ankicommunity/anki-sync-server-rs/releases/latest)[![](https://img.shields.io/github/last-commit/ankicommunity/anki-sync-server-rs)]()

A cross-platform, Anki sync server.

This is a rust (still sqlite c library backed) take on anki sync server (for a mature python one see [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server)).


## How to use anki-sync-server-rs

1. Install anki-sync-server-rs, see underneath or [INSTALL.md](docs/INSTALL.md) for more information.
2. Configure it (see template configuration `Settings.toml`)
3. Run the server
4. Add user to the server
5. Configure your client to sync with the custom server, see [here](docs/ANKI_CLIENTS_SETUP.md)

For https setup and support see [certificate setup](docs/CERTS.md).
See [reverse proxy setup](docs/REVERSE_PROXY.md) for setting up a reverse proxy in front of the sync server.


### Quickstart guide

#### Linux

1. Grab binary from github [releases](https://github.com/ankicommunity/anki-sync-server-rs/releases) and unpack it, `ankisyncd_x.x.x-linux_x86_64_xxx.tar.gz` for x86-64 linux, `ankisyncd-x.x.x-armv6_muslc.tar.gz` for armv6 32bit NEON board under linux) or even better build it from source (see `INSTALL.md`)
2. Tweak the configuration `ankisyncd.toml` to your liking (if you want to use it)
3. Run server `./ankisyncd` (use `--config ANKISYNCD_CONFIG_PATH` if needed)
4. Add user `./ankisyncd user --add username password` (use `./ankisyncd user --help` for more on user management, add `--config ANKISYNCD_CONFIG_PATH` to the command when using a config file)
5. Enjoy!

#### Windows

1. Grab binray from github [releases](https://github.com/ankicommunity/anki-sync-server-rs/releases) and unpack it,`ankisyncd_x.x.x-windows_x86_64.zip` for x86-64 windows.
2. Tweak the configuration `ankisyncd.toml` to your liking (if you want to use it)
3. Run server with the defaults (double click) `ankisyncd.exe` (run `ankisyncd.exe --config ANKISYNCD_CONFIG_PATH` from the command line to use a specific config file)
4. Add user using command line `./ankisyncd.exe user --add username password` (use `./ankisyncd.exe user --help` for more on user management, add `--config ANKISYNCD_CONFIG_PATH` to the command when using a config file)
5. Enjoy!

#### MacOS

1. Grab binray from github [releases](https://github.com/ankicommunity/anki-sync-server-rs/releases) and unpack it,`ankisyncd_x.x.x-macOS_x86_64.zip` for x86-64 MacOS.
2. Tweak the configuration `Settings.toml` to your liking
3. Run server (double click) `ankisyncd`
4. Add user using command line `./ankisyncd user --add username password` (use `./ankisyncd.exe user --help` for more on user management)
5. Enjoy!

### Containerized build

For easier development/build/deployment without installing any toolchain see [containerized build](docs/CONTAINER.md)

## How to contribute

See [CONTRIBUTING.md](CONTRIBUTING.md).

All contributions must be licensed under AGLP-v3.0 to comply with the license of the anki code used as the base of this project.

## License

See [LICENSE](LICENSE)


## Compatibility

### Server

It should work on any tier 1/2 platform of the rust ecosystem.
But have only been tested on the following.

#### Windows

Win 10 64bits

#### Linux

|machine|ENV|
|----|----|
|x86_64|Windows wsl2,tested|
|aarch64(arm64)|cross-compiled on wsl2(ubuntu),tested on ubuntu aarch64 and termux|
|armv7(arm32)|cross-compiled on wsl2(ubuntu)|


### Client

|tested anki versions|2.1.15,2.1.28,2.1.35,2.1.50|
|----|----|
|tested process| import a collection of decks and upload to server|


## REFERENCE
ankisyncd architecture or apis depend on [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server) and
[ankitects/anki](https://github.com/ankitects/anki).
Sync APIs are based on anki/rslib 2.1.46

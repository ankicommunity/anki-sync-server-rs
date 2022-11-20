<div align="center">
# anki-sync-server-rs

[![License](https://img.shields.io/github/license/ankicommunity/anki-sync-server-rs)](https://github.com/ankicommunity/anki-sync-server-rs/blob/master/LINCENSE)[![Github status](https://img.shields.io/github/checks-status/ankicommunity/anki-sync-server-rs/master?label=github%20status)](https://github.com/ankicommunity/anki-sync-server-rs/actions)[![Github contributors](https://img.shields.io/github/contributors/ankicommunity/anki-sync-server-rs?label=github%20contributors)](https://github.com/ankicommunity/anki-sync-server-rs/graphs/contributors)[![DockerHub version](https://img.shields.io/docker/v/ankicommunity/anki-sync-server-rs?label=dockerhub%20version&sort=date)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)[![DockerHub pulls](https://img.shields.io/docker/pulls/ankicommunity/anki-sync-server-rs)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)[![DockerHub stars](https://img.shields.io/docker/stars/ankicommunity/anki-sync-server-rs)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)
[![](https://img.shields.io/github/v/release/ankicommunity/anki-sync-server-rs)](https://github.com/ankicommunity/anki-sync-server-rs/releases/latest)[![](https://img.shields.io/github/last-commit/ankicommunity/anki-sync-server-rs)]()[![Gitter](https://badges.gitter.im/ankicommunity/community.svg)](https://gitter.im/ankicommunity/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

[简体中文](README_CN.md)|[English](README.md)

</div>
A cross-platform Anki sync server.

This is a rust (still sqlite c library backed) take on anki sync server (for a mature python one see [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server)).

## Quickstart guide
### Installing (binary)
1. Grab binary from github [releases](https://github.com/ankicommunity/anki-sync-server-rs/releases) and unpack it, each platform has its corresponding tag (e.g. `windows_x86_64` for Windows 64bit,details see [support platform](docs/PLATFORM.md) ) .
2. Add user
For linux users or macOS users,run,
```
 ./ankisyncd user --add username password
```
for Windows users,open a terminal in the same folder where the binary is in and run,
```
 ./ankisyncd.exe user --add username password
```
If you want to perform other operations ,such as deleting users or changing password of one user,run with help argument for more details,
```
 ./ankisyncd user --help
```
3. Run server `./ankisyncd` (for Windows users,you can just double click the binary for a quick start).
4. Enjoy!

### Installing (Docker)
details see [Docker](docs/CONTAINER.md)

You can also build from source code [Install](docs/INSTALL.md) or build docker image from source [DockerBuild](docs/CONTAINER.md).
## Set up Anki (Clients)
### Anki 2.1(install add-on from ankiweb)

1. Go to `Tools -> Add-ons`
2. On the add-on window, click on `Get Add-ons` and fill in the textbox with the code `358444159`
3. You get add-on `custom sync server redirector`, choose it. Then click `config` on the bottom right
4. Apply your server IP address
5. Restart Anki

### AnkiDroid

Go to `Advanced -> Custom sync server` (Go to `Settings` -> `Sync` -> `Custom sync server` in  2.16 and above)

Unless you have set up a reverse proxy to handle encrypted connections, use `http` as the protocol. The port will be either the default `27701`, or whatever you have specified in `ankisyncd.toml` (if using a reverse proxy, whatever port you configured to accept the front-end connection).

Use the same base url for both the `Sync url` and the `Media sync url`, but append `/msync` to the `Media sync url`. Do **not** append `/sync` to the `Sync url` (Note: This is not the case any more in above version 2.16,include 2.16).
Take IP address `192.0.0.0` for example and use default port `27701` with `http` protocol,the corresponsding urls are,

Sync url:`http://192.0.0.0:27701`
Media sync url: `http://192.0.0.0:27701/msync`

In above version 2.16,
Sync url:`http://192.0.0.0:27701/sync/`
Media sync url: `http://192.0.0.0:27701/msync/`

Even though the AnkiDroid login interface will request an email address, this is not actually required; it can simply be the username you configured with `ankisyncd user -a`.

For https setup and support see [certificate setup](docs/CERTS.md) (Note: in 2.16 and above,Ankidroid could supprt http connection once more).
See [reverse proxy setup](docs/REVERSE_PROXY.md) for setting up a reverse proxy in front of the sync server.

## How to contribute

See [CONTRIBUTING.md](CONTRIBUTING.md).

All contributions must be licensed under AGLP-v3.0 to comply with the license of the anki code used as the base of this project.

## License

See [LICENSE](LICENSE)

## Compatibility
When the server made its first appearance,we have done some tests,details see [TEST](docs/TEST_SERVER_CLIENT.md)
## Configuration
### Env vars
Ankidyncd supports two Env variables to add users,`ANKISYNCD_USERNAME`,`ANKISYNCD_PASSWORD`.
|Key|Value|
|-|-|
|ANKISYNCD_USERNAME|username,non-empty|
|ANKISYNCD_PASSWORD|password,non-empty|

### Optional Server Configuration
If you want to change the folder where sync data reside or change the listening port,you can modify the configuration file `ankisyncd.toml`,and then run server,
```
./ankisyncd  --config /path/to/ankisyncd.toml
```

## REFERENCE
ankisyncd architecture or apis depend on [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server) and
[ankitects/anki](https://github.com/ankitects/anki).
Sync APIs are initially based on anki/rslib 2.1.46.Note: This project is heavily dependent on upstream project `Anki`,that is,if the project Anki is no longer accessible,this project might be malfunctional and abandoned.

<div align="center">

# anki-sync-server-rs
![GitHub repo size](https://img.shields.io/github/repo-size/ankicommunity/anki-sync-server-rs)
[![License](https://img.shields.io/github/license/ankicommunity/anki-sync-server-rs)](https://github.com/ankicommunity/anki-sync-server-rs/blob/master/LINCENSE)[![Github status](https://img.shields.io/github/checks-status/ankicommunity/anki-sync-server-rs/master?label=github%20status)](https://github.com/ankicommunity/anki-sync-server-rs/actions)[![Github contributors](https://img.shields.io/github/contributors/ankicommunity/anki-sync-server-rs?label=github%20contributors)](https://github.com/ankicommunity/anki-sync-server-rs/graphs/contributors)[![DockerHub version](https://img.shields.io/docker/v/ankicommunity/anki-sync-server-rs?label=dockerhub%20version&sort=date)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)[![DockerHub pulls](https://img.shields.io/docker/pulls/ankicommunity/anki-sync-server-rs)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)[![DockerHub stars](https://img.shields.io/docker/stars/ankicommunity/anki-sync-server-rs)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)
[![](https://img.shields.io/github/v/release/ankicommunity/anki-sync-server-rs)](https://github.com/ankicommunity/anki-sync-server-rs/releases/latest)[![](https://img.shields.io/github/last-commit/ankicommunity/anki-sync-server-rs)]()[![Gitter](https://badges.gitter.im/ankicommunity/community.svg)](https://gitter.im/ankicommunity/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)
 [![Downloads](https://img.shields.io/github/downloads/ankicommunity/anki-sync-server-rs/total?label=Release%20Download)](https://github.com/ankicommunity/anki-sync-server-rs/releases/latest)

[简体中文](README_CN.md)|[English](README.md)

</div>
A cross-platform Anki sync server.

This is a rust (still sqlite c library backed) take on anki sync server ,which keep track of the
official sync server.

If you would like to use the sync server bundled with the Anki client or run the server via python,go to [guide](https://docs.ankiweb.net/sync-server.html) for more.

## Quickstart guide
### Installing (binary)
1. Grab binary from github [releases](https://github.com/ankicommunity/anki-sync-server-rs/releases) and unpack it, each platform has its corresponding tag (e.g. `windows_x86_64` for Windows 64bit,details see [support platform](docs/PLATFORM.md) ) ,enter the decompressed folder.
2. Add user

For linux users or macOS users,run,
```
 ./ankisyncd user --add username password
```
for Windows users,open a terminal in the folder and run,
```
 ./ankisyncd.exe user --add username password
```
If you want to perform other operations ,such as deleting users or changing the password of one user,run with the `--help` flag for more details,
```
 ./ankisyncd user --help
```
3. Run server `./ankisyncd` (for Windows users,you can just double click the binary for a quick start).
4. Enjoy!

### Installing (Docker)
details see [Docker](docs/CONTAINER.md)

You can also build the binary from source code [Install](docs/INSTALL.md) or build a docker image from the source [DockerBuild](docs/CONTAINER.md).
## Set up Anki (Clients)
### Anki 2.1
#### >= 2.1.57
Due to the software update,Now Anki supports sync custom server settings inside the client.
1. Go to `Tools ->Preferences--<Syncing`
2. see below and enter your server address in the blank labled `self-hosted sync server`.
Here is an example.If your server address is `192.0.0.1`,then the content to fill in is `http://192.0.0.1:27701/`  
3. Restart Anki
#### <2.1.57 ((install add-on from ankiweb)
1. Go to `Tools -> Add-ons`
2.  click on the button labeled `Get Add-ons` and  enter the code `358444159`.
3. You get add-on `custom sync server redirector`, choose it. Then click on the `config` button in the bottom right corner.
4. Apply your server IP address
5. Restart Anki
### AnkiMobile
It seems that Ankimobile now has the ability to sync against self-hosted sync server.At least for  Ankimobile 2.0.90(20090.2),A post from [A user has reported in Anki forum](https://forums.ankiweb.net/t/ankimobile-self-sync-server-failure-the-one-bundled-in-version-2-1-60-qt6/27862).
As for the detailed steps,we will be happy to accept a PR about how to configure AnkiMobile to enable custom sync server If some one is using AnkiMobile and would be kind enough.
When things do not go as expected,refer to the text:
> If you're using AnkiMobile and are unable to connect to a server on your local network, please go into the iOS settings, locate Anki near the bottom, and toggle "Allow Anki to access local network" off and then on again.

[From Anki tutorial](https://docs.ankiweb.net/sync-server.html#client-setup)
### AnkiDroid

Go to `Advanced -> Custom sync server` (Go to `Settings` -> `Sync` -> `Custom sync server` in  2.16 and newer versions)

Unless you have set up a reverse proxy to handle encrypted connections, use `http` as the protocol. The port will be either the default `27701`, or whatever you have specified in `ankisyncd.toml` (if using a reverse proxy, whatever port you configured to accept the front-end connection).

Use the same base url for both the `Sync url` and the `Media sync url`, but append `/msync` to the `Media sync url`. Do **not** append `/sync` to the `Sync url` (Note: This is not the case any more in 2.16 and newer versions).

Take IP address `192.0.0.0` for example and use default port `27701` with `http` protocol,the corresponsding urls are,

Sync url:`http://192.0.0.0:27701`

Media sync url: `http://192.0.0.0:27701/msync`

In 2.16 and newer versions,

Sync url:`http://192.0.0.0:27701/sync/`

Media sync url: `http://192.0.0.0:27701/msync/`

Even though the AnkiDroid login interface will request an email address, this is not actually required; it can simply be the username you configured with `ankisyncd user -a`.

For https setup and support see [certificate setup](docs/CERTS.md) (Note: in 2.16 and newer versions,Ankidroid could supprt http connection once more).
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
Ankidyncd supports setting environment variables to add accounts,`ANKISYNCD_USERNAME`,`ANKISYNCD_PASSWORD`.
|Key|Value|
|-|-|
|ANKISYNCD_USERNAME|username,non-empty if set|
|ANKISYNCD_PASSWORD|password,non-empty if set|

### Optional Server Configuration
If you want to change the location where sync data is stored, or change the listening port,you can modify the configuration file `ankisyncd.toml`,and then run server,
```
./ankisyncd  --config /path/to/ankisyncd.toml
```

## REFERENCE
ankisyncd architecture or apis depend on [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server) and
[ankitects/anki](https://github.com/ankitects/anki).
Sync APIs are initially based on anki/rslib 2.1.46.We almost replicated the media synchronization implementation logic in `anki-sync-server`.And this project is heavily dependent on upstream project `Anki`,that is,if the project Anki is no longer accessible,this project might be malfunctional and abandoned.

SInce 2.1.57，this project keeps track of the process of Anki sync server.

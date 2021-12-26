[![](https://img.shields.io/github/v/release/ankicommunity/anki-sync-server-rs)](https://github.com/ankicommunity/anki-sync-server-rs/releases/latest)[![](https://img.shields.io/github/last-commit/ankicommunity/anki-sync-server-rs)]()

This one is taken as a Rust version
of  [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server).
<br>

## Install 

### run from built binary package
download executables from [releases](https://github.com/ankicommunity/anki-sync-server-rs/releases)
#### Windows(x86_64/i686)
After decompression,an account is required,following instructions
below on how to create an account.
Then,double click `ankisyncd.exe` to get started.
#### Linux
1. Fisrt of all,decompression,
```
tar -zxvf ankisyncd_xxx.tar.gz
```
2. create an account following instructions
below.
3. run from terminal,
```
./ankisyncd
```
Currently supported Linux platforms
|machine|ENV|
|----|----|
|x86_64|Windows wsl2(need testing )|
|aarch64(arm64)|cross-compiled on wsl2(ubuntu),tested on ubuntu aarch64 and termux|
|armv7(arm32)|cross-compiled on wsl2(ubuntu)|
#### MacOS(x86_64,need testing)
After decompression,an account is required,following instructions
below on how to create an account.
### containerized build (docker) and run
allow for easy development/build without installing any 
toolchain,more see [containerized build](https://github.com/ankicommunity/anki-sync-server-rs/blob/master/docs/container.md)
### build from source
1. make sure Rust and its toolchains are installed.
follow [this link](https://www.rust-lang.org/tools/install) using rustup to install.
2. clone our repo and enter into the folder, run build command
`cargo build --release`
## Setting up Anki

#### Anki 2.1(install add-on from ankiweb)
Tools -> Add-ons

1. on add-on window,click `Get Add-ons` and fill in the textbox with the code  `358444159`

2. there,you get add-on `custom sync server redirector`,choose it.Then click `config`  below right

3. apply your server ip address ,restart Anki

#### AnkiDroid

Advanced → Custom sync server

Unless you have set up a reverse proxy to handle encrypted connections, use `http` as the protocol. The port will be either the default, 27701, or whatever you have specified in `Settings.toml` (or, if using a reverse proxy, whatever port you configured to accept the front-end connection).

Use the same base url for both the `Sync url` and the `Media sync url`, but append `/msync` to the `Media sync url`. Do **not** append `/sync` to the `Sync url`.

Even though the AnkiDroid interface will request an email address, this is not required; it will simply be the username you configured with `ankisyncd.exe user -a`.
#### encrypted HTTP connection
Due to Android policy change,some ankidroid versions need
https transportation.Though we recommend reverse proxies such as `Nginx` and `Caddy` to perform secure connection,
ankisyncd allow for embeded self-signed certicate 
used in LAN environment.If you want to use that feature,you must run cargo build command with flag
 `--feature rustls`.Then open `Settings.toml` with text
editor,modify following lines:
```
#make ssl_enable true
ssl_enable=false
# put cert and key file path 
cert_file=""
key_file=""
```
## Account Management

#### use subcommand of ankisyncd

enter into ankisyncd account management
i.e.create user account

```
ankisyncd.exe user -a xiaoshiyilang iampass
```

more subcommand see help `ankisyncd.exe -h`

### TODO

- [ ] error handle
- [ ]  fix incorrect log time (when running on cross-compiled
binary)
- [ ]  add github repo link to log info
- [ ] add username to log info
- [ ] wiki writing
### Compatibility
|tested anki versions|2.1.15,2.1.28,2.1.35,2.1.50|
|----|----|
|tested process| import a collection of decks and upload to server|

### REFERENCE
ankisyncd architecture or apis depend on [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server) and
[ankitects/anki](https://github.com/ankitects/anki).
Sync APIs are based on anki/rslib 2.1.46
This one is taken as an anki-sync-server implementation in Rust
of  [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server).
<br>

## Install 

### run from built binary package
download executables from releases
#### Windows(x86_64)
After decompression,an account is required,following instructions
below on how to create an account.
Then,double click `ankisyncd.exe` to get started.
#### LInux
##### x86_64
Warn:need testing if it's working on other computers,as this build
seems a dynamically linked bibary
- decompression
`tar -zxvf ankisyncd_linux_x86_64.tar.gz`
- probably,create an account first following instructions
below.
- run
`./ankisyncd`
##### aarch64
- decompression
`tar -zxvf ankisyncd_aarch64.tar.gz`
- probably,create an account first following instructions
below.
- run
`./ankisyncd`
### build from source
1. make sure Rust and its toolchains are installed.
follow [this link](https://www.rust-lang.org/tools/install) using rustup to install.
2. open terminal,clone github repo.
`git clone https://github.com/dobefore/anki-sync-server-rs.git `
3. run build command
`cargo build --release`
## Setting up Anki

#### Anki 2.1(install add from ankiweb)
Tools -> Add-ons

1. on add-on window,click `Get Add-ons` and fill in the textbox with the code  `358444159`

2. there,you get add-on `custom sync server redirector`,choose it.Then click `config`  below right

3. apply your server ip address 

#### AnkiDroid

Advanced → Custom sync server

Unless you have set up a reverse proxy to handle encrypted connections, use `http` as the protocol. The port will be either the default, 27701, or whatever you have specified in `ankisyncd.conf` (or, if using a reverse proxy, whatever port you configured to accept the front-end connection).

Use the same base url for both the `Sync url` and the `Media sync url`, but append `/msync` to the `Media sync url`. Do **not** append `/sync` to the `Sync url`.

Even though the AnkiDroid interface will request an email address, this is not required; it will simply be the username you configured with `ankisyncctl.exe adduser`.

## Account Management

#### option1

enter into ankisyncd account management

`ankisyncd.exe U`

and then followning the instructions

#### option2

use separate command line tool `ankisyncctl.exe` 

ie:create user account

`ankisyncctl.exe adduser zhigufei password`

more operations can be found by query help

`ankisyncctl -h`

### TODO

- [x] allow self-signed certificate used in https in Intranet environment
- [ ] error handle
- [x] anki versions test
- [x]  automatically add user account into auth ab when account
  is not empty in Settings.toml
- [ ]  builds for Linux and MacOS
### Compatibility
|tested anki versions|2.1.15,2.1.28,2.1.35,2.1.50|
|----|----|
|tested process| import a collection of decks and upload to server|

### REFERENCE
ankisyncd architecture or apis depend on [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server) and
[ankitects/anki](https://github.com/ankitects/anki).
Sync APIs are based on anki/rslib 2.1.46
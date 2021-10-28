custom sync server implemented in Rust.
### download binary package
### build from source

### TODO
- add ssl certificate verify
- error handle
- anki versions test
- automatically add user account into auth ab when account
is not empty in Settings.toml
- add shell script for  build from source on linux and windows
- make deb for linux

### REFERENCE
ankisyncd architecture or apis depend on [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server) and
[ankitects/anki](https://github.com/ankitects/anki).
Sync APIs are based on anki/rslib 2.1.46

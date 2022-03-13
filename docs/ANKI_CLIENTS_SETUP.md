## Setting up Anki Clients

#### Anki 2.1(install add-on from ankiweb)

1. Go to `Tools -> Add-ons`
2. On the add-on window, click on `Get Add-ons` and fill in the textbox with the code `358444159`
3. You get add-on `custom sync server redirector`, choose it. Then click `config` on the bottom right
4. Apply your server ip address
5. Restart Anki

#### AnkiDroid

Go to `Advanced -> Custom sync server`

Unless you have set up a reverse proxy to handle encrypted connections, use `http` as the protocol. The port will be either the default `27701`, or whatever you have specified in `Settings.toml` (if using a reverse proxy, whatever port you configured to accept the front-end connection).

Use the same base url for both the `Sync url` and the `Media sync url`, but append `/msync` to the `Media sync url`. Do **not** append `/sync` to the `Sync url`.

Even though the AnkiDroid interface will request an email address, this is not required; it will simply be the username you configured with `ankisyncd user -a`.

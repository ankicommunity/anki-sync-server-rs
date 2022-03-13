# HTTPS setup

Due to Android policy change, some ankidroid versions need an https transport.
Ankisyncd allow the use of self-signed certicates
that enable more secure connection
such as in semi-open LAN environment.
This requires the syncserver to be compiled with the `rustls` feature (pass `--feature rustls` to cargo when building).

Open `Settings.toml` with a text editor
and modify following lines to enable and set certificates paths:
```
#make ssl_enable true
ssl_enable=false
# put cert and key file path 
cert_file=""
key_file=""
```

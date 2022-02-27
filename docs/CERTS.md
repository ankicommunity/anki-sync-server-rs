# HTTPS setup

Due to Android policy change, some ankidroid versions need an https transport.
Ankisyncd allow the use of self-signed certicates
that enable more secure connection
such as in semi-open LAN environment.

Open `Settings.toml` with a text editor
and modify following lines to enable and set certificates paths:
```
#make ssl_enable true
ssl_enable=false
# put cert and key file path 
cert_file=""
key_file=""
```

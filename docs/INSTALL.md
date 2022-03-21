# Install

## Grab release

See Quickstart in [README.md](../README.md).

## Build from source 

1. make sure Rust and its toolchains are installed.
In doubt use rustup as proposed in [this link](https://www.rust-lang.org/tools/install).
2. clone our repo and enter into the folder
3. Populate anki lib by running `scripts/clone_patch_anki`
4. run build command `cargo build --release`
5. The resulting binary is available in `target/release/`

## Install as a systemd unit

We suppose that the sync server is installed in `/usr/bin/ankisyncd`.
Install the configuration file in `/etc/ankisyncd.toml`
with root dir set to `/var/lib/ankisyncd/`.

Create a new system user and group named `anki` (using `useradd`).

Create and change ownership of the root dir: `mkdir -p /var/lib/ankisyncd/ && chmod -R o-a /var/lib/ankisyncd/ && chown -R anki:anki /var/lib/ankisyncd/`

Then populate the secure service file in `/etc/systemd/system/ankisyncd.service`
```
[Unit]
Description=Anki sync server daemon
After=network-online.target
# If reverse proxy start after it
#After=network-online.target nginx.service
Wants=network-online.target
[Service]
Type=exec
ExecStart=/usr/bin/ankisyncd -c /etc/ankisyncd.toml
User=anki
Group=anki
SyslogIdentifier=ankisyncd
WorkingDirectory=/var/lib/ankisyncd/
PrivateTmp=true
PrivateDevices=true
CapabilityBoundingSet=
AmbientCapabilities=
ProtectSystem=strict
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
ProtectClock=true
ProtectHostname=true
ProtectHome=tmpfs
ProtectKernelLogs=true
ProtectProc=invisible
ProcSubset=pid
PrivateNetwork=false
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
IPAddressAllow=any
SystemCallArchitectures=native
SystemCallFilter=@system-service
SystemCallFilter=~@privileged @resources @obsolete
RestrictSUIDSGID=true
RemoveIPC=true
NoNewPrivileges=true
RestrictRealtime=true
RestrictNamespaces=true
LockPersonality=true
PrivateUsers=true
MemoryDenyWriteExecute=false

[Install]
WantedBy=multi-user.target
```

Reload services list `systemctl daemon-reload`.

Enable and start sync server `systemctl enable ankisyncd && systemctl start ankisyncd`.

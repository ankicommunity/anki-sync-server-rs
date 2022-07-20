# Containers 
buit images from docker hub for arm64 and amd64 are available, or you can build by yourselves.

This file contains:
1. how to run binary from built image
2. How to use container to build binary
3. How to run binary in a container

In this manual we will use `podman` command for containers creation/management but it can seamlessly be replaced with `docker` every time it is used.

The `Containerfile` at the root of the repository controls the build process.
## Pull image from hub and run in container
1. pull image
```
docker pull ankicommunity/anki-sync-server-rs:latest
```
2. run it in background (you can specify the container name by passing `--name=ankisyncd` or use default name)
```
docker run -d -it ankicommunity/anki-sync-server-rs:latest
```
3. add user
bring up the shell of the `ankisyncd` container(or default container name) and run command
```
docker exec -it ankisyncd /bin/bash
ankisyncd user -a username password
exit
```
## Building in container, running on host

1. In the root of the repository run: 
```
podman build -t anki-sync-server-rs/builder:latest .
```
2. Then exfiltrate the binary from the container:
```
podman run --rm --entrypoint cat anki-sync-server-rs/builder:latest /usr/local/bin/ankisyncd > ankisyncd
```
3. Use the `ankisyncd` binary obtained as usual


## Building and running in container

1. Build the container: 
```
podman build -t anki-sync-server-rs/runner:latest .
```
2. Run it in foreground: 
```
podman run -it anki-sync-server-rs/runner:latest
```

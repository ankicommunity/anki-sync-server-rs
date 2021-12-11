# Containers 

This file contains:

1. How to use container to build binary
2. How to run binary in a container

In this manual we will use `podman` command for containers creation/management but it can seamlessly be replaced with `docker` every time it is used.

The `Containerfile` at the root of the repository controls the build process.

## Building in container, running on host

1. In the root of the repository run:
 ``` podman build -t anki-sync-server-rs/builder:latest .```
2. Then exfiltrate the binary from the container: 
```podman run --rm --entrypoint cat anki-sync-server-rs/builder:latest /usr/local/bin/ankisyncd > ankisyncd```
3. Use the `ankisyncd` binary obtained as usual


## Building and running in container

1. Build the container: ```podman build -t anki-sync-server-rs/runner:latest .```
2. Run it in foreground: ```podman run -it anki-sync-server-rs/runner:latest```



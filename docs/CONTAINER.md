# Containers 
pre-built images from docker hub for arm64 and amd64 are available, or you can build it by yourselves.

In this manual we will use `docker` command for containers creation/management but it can seamlessly be replaced with `podman` every time it is used.

The `Dockerfile` at the root of the repository controls the build process.
## Pull images from DockerHub and run in container
1. pull image
```
docker pull ankicommunity/anki-sync-server-rs:latest
```
2. run it in background (you can specify the container name by passing `--name=ankisyncd` or use default name).And,you can pass env vars to following command line to add users,for example,following part of env vars will add an account whose username is `test` and password is `123456`.
```
docker run -d -it --name=ankisyncd -e ANKISYNCD_USERNAME=test -e ANKISYNCD_PASSWORD=123456 ankicommunity/anki-sync-server-rs:latest
```
3. add user
If env variables are already set ,which means the account has been added,there is no need to do this step.If not,bring up the shell of the `ankisyncd` container(or default container name) and run command
```
docker exec -it ankisyncd /bin/bash
ankisyncd user -a username password
exit
```
## Building in container, running on host

1. In the root of the repository run: 
```
docker build -t anki-sync-server-rs/builder:latest .
```
2. Then exfiltrate the binary from the container:
```
docker run --rm --entrypoint cat anki-sync-server-rs/builder:latest /usr/local/bin/ankisyncd > ankisyncd
```
3. Use the `ankisyncd` binary obtained as usual


## Building and running in container

1. Build the container: 
```
docker build -t anki-sync-server-rs/runner:latest .
```
2. Run it in foreground: 
```
docker run -it anki-sync-server-rs/runner:latest
```

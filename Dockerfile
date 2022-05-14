FROM rust:latest as builder
WORKDIR /usr/src/anki-sync-server-rs
# copy from host to container
COPY . .
# prost-build failed for armv7h https://github.com/ankicommunity/anki-sync-server-rs/issues/22 
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --assume-yes protobuf-compiler git
RUN scripts/clone_patch_anki
RUN cargo build --release  && cp ./target/release/ankisyncd . && cargo clean

FROM debian:stable-slim as runner
#RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/anki-sync-server-rs/ankisyncd /usr/local/bin/ankisyncd
RUN chmod +x /usr/local/bin/ankisyncd 
# WORKDIR /app means, when you log into the shell of container，
# you will be in the /app directory of the container by default.
WORKDIR /app
# https://linuxhint.com/dockerfile_volumes/
# persist data with a named volume https://docs.docker.com/get-started/05_persisting_data/
VOLUME /app
COPY --from=builder /usr/src/anki-sync-server-rs/scripts/ankisyncd.toml /app/ankisyncd.toml
CMD ["ankisyncd", "-c","/app/ankisyncd.toml"]
EXPOSE 27701

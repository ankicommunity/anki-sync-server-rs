# FROM strophy/protoc:4.0.1 as protocc
# RUN protoc -h
# FROM rust:latest as builder
FROM debian:stable-slim 
WORKDIR /app
# copy from host to container
COPY . /home
# COPY --from=protocc /usr/bin/protoc /usr/bin/protoc
# prost-build failed for armv7h https://github.com/ankicommunity/anki-sync-server-rs/issues/22 

# RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --assume-yes protobuf-compiler git
# RUN scripts/clone_patch_anki
# RUN cargo build --release  
# RUN cp ./target/release/ankisyncd . && cargo clean

#RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
RUN mv /home/binary/`arch`-unknown-linux-musl-ankisyncd /usr/local/bin/ankisyncd
RUN ls /home
RUN chmod +x /usr/local/bin/ankisyncd 
# WORKDIR /app means, when you log into the shell of container，
# you will be in the /app directory of the container by default.
# WORKDIR /app
# https://linuxhint.com/dockerfile_volumes/
# persist data with a named volume https://docs.docker.com/get-started/05_persisting_data/
VOLUME /app
COPY  /home/scripts/ankisyncd.toml /ankisyncd.toml
COPY  /home/scripts/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
CMD ["sh", "/entrypoint.sh"]
EXPOSE 27701

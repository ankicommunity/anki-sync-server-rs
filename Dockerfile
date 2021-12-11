FROM rust:latest as builder
WORKDIR /usr/src/anki-sync-server-rs
# copy from host to container
COPY . .
RUN cargo build --release  

FROM debian:stable-slim as runner
#RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/anki-sync-server-rs/target/release/ankisyncd /usr/local/bin/ankisyncd
RUN chmod +x /usr/local/bin/ankisyncd
CMD ["ankisyncd"]

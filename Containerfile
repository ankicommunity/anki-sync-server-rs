FROM rust:latest as builder
WORKDIR /usr/src/anki-sync-server-rs
COPY . .
RUN cargo install --path .

FROM debian:stable-slim as runner
#RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/ankisyncd /usr/local/bin/ankisyncd
CMD ["ankisyncd"]

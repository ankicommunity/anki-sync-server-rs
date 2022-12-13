FROM debian:stable-slim 
WORKDIR /app
# copy from host to container
COPY binary .

RUN cp /app/binary/ankisyncd.toml /app/binary/entrypoint.sh /
RUN mv /app/binary/`arch`-unknown-linux-musl-ankisyncd /usr/local/bin/ankisyncd
RUN chmod +x /usr/local/bin/ankisyncd && cp /app/binary/ankisyncd.toml /app && rm -fr binary
# WORKDIR /app means, when you log into the shell of container，
# you will be in the /app directory of the container by default.
# https://linuxhint.com/dockerfile_volumes/
# persist data with a named volume https://docs.docker.com/get-started/05_persisting_data/
VOLUME /app

RUN chmod +x /entrypoint.sh
CMD ["sh", "/entrypoint.sh"]
EXPOSE 27701

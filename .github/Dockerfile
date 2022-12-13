FROM debian:stable-slim 
WORKDIR /app
# copy from host to container
COPY binary binary

RUN cp /app/binary/ankisyncd.toml /ankisyncd.toml && cp /app/binary/ankisyncd.toml /app/ankisyncd.toml && cp /app/binary/entrypoint.sh /entrypoint.sh
RUN mv /app/binary/`arch`-unknown-linux-musl-ankisyncd /usr/local/bin/ankisyncd
RUN chmod +x /usr/local/bin/ankisyncd  && rm -fr binary
# WORKDIR /app means, when you log into the shell of container，
# you will be in the /app directory of the container by default.
# https://linuxhint.com/dockerfile_volumes/
# persist data with a named volume https://docs.docker.com/get-started/05_persisting_data/
VOLUME /app

RUN chmod +x /entrypoint.sh
CMD ["sh", "/entrypoint.sh"]
EXPOSE 27701

#!/bin/sh
if [ ! -f /app/ankisyncd.toml ]; then
	cp -u /ankisyncd.toml /app/ankisyncd.toml
fi
/usr/local/bin/ankisyncd -c /app/ankisyncd.toml

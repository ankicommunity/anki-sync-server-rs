### Server

It should work on any tier 1/2 platform of the rust ecosystem.
But have only been tested on the following.

#### Windows

Win 10 64bits

#### Linux

|machine|ENV|
|----|----|
|x86_64|Windows wsl2,tested|
|aarch64(arm64)|cross-compiled on wsl2(ubuntu),tested on ubuntu aarch64 and termux|
|armv7(arm32)|cross-compiled on wsl2(ubuntu)|


### Client

|tested anki versions|2.1.15,2.1.28,2.1.35,2.1.50|
|----|----|
|tested process| import a collection of decks and upload to server|
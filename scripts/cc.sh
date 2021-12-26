# for cross compile use

# cp cc.sh and config to dir ..,then change dir to ..

# for aarch64
# echo "cross-compile for aarch64"
# file1="target/aarch64-unknown-linux-musl/release/ankisyncd"
# if [ -f $file1 ];then
#  echo "$file1 exists"
# else
# # add env var
# export PATH="$HOME/aarch64-linux-musl-cross/bin:$PATH"
# # use compiled openssl
# export OPENSSL_LIB_DIR=/home/ubuntu/openssl_aarch64_musl
# export OPENSSL_INCLUDE_DIR=/home/ubuntu/openssl_aarch64_musl/include
# export OPENSSL_STATIC=true
# # cross build for aarch64
# cargo build --target=aarch64-unknown-linux-musl --release
# fi
version="0.1.3"
# for armv7
echo "cross-compile for armv7"
file2="target/armv7-unknown-linux-musleabihf/release/ankisyncd"
if [ -f $file2 ];then
 echo "$file2 exists"
else
# set CC locenv var
export PATH="$HOME/rpitools/arm-bcm2708/arm-rpi-4.9.3-linux-gnueabihf/bin:$PATH"
export CC=arm-linux-gnueabihf-gcc
# add env var
export PATH="$HOME/arm-linux-musleabihf-cross/bin:$PATH"
# use compiled openssl
export OPENSSL_LIB_DIR=/home/ubuntu/openssl-armv7/lib
export OPENSSL_INCLUDE_DIR=/home/ubuntu/openssl-armv7/include
export OPENSSL_STATIC=true
# cross build for armv7 enable feature --features rustls
 cargo build --target armv7-unknown-linux-musleabihf --release

mkdir ankisyncd-arm
cp target/armv7-unknown-linux-musleabihf/release/ankisyncd ankisyncd-arm/
cp Settings.toml ankisyncd-arm/
tar -czvf ankisyncd-$version-arm.tar.gz ankisyncd-arm/
mv ankisyncd-$version-linux-arm.tar.gz ~
fi

#for x86-64
echo "cross-compile for x64"
# add env var
export PATH="$HOME/x86_64-linux-musl-cross/bin:$PATH"
export CC=

export OPENSSL_LIB_DIR=/home/ubuntu/openssl_x64
export OPENSSL_INCLUDE_DIR=/home/ubuntu/openssl_x64/include
export OPENSSL_STATIC=true
#  enable feature --features rustls
cargo build --release --target=x86_64-unknown-linux-musl

mkdir ankisyncd-linux
cp target/x86_64-unknown-linux-musl/release/ankisyncd ankisyncd-linux/
cp Settings.toml ankisyncd-linux/
tar -czvf ankisyncd-$version-linux.tar.gz ankisyncd-linux/
mv ankisyncd-$version-linux.tar.gz ~
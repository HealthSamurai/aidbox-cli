# rm -r target
cargo build --release
strip target/release/aidbox

mkdir -p bin
cp target/release/aidbox bin/aidbox.macos

docker run -v $PWD:/volume --rm -ti clux/muslrust  bash -c 'cargo build --release && strip target/x86_64-unknown-linux-musl/release/aidbox'
cp target/x86_64-unknown-linux-musl/release/aidbox bin/aidbox.linux

ls -lah bin

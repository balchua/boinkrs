# Boink in Rust

This is a port of Boink to Rust.

## Build requirements

On Ubuntu
Need to install `pkg-config`

``` shell
sudo apt install pkg-config
sudo apt install libssl-dev
```


## Code cov

Go to [grcov](https://github.com/mozilla/grcov)

Follow how to install `grcov` [here](https://github.com/mozilla/grcov#how-to-get-grcov)

``` shell
$ CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='./target/coverage/raw/cargo-test-%p-%m.profraw' cargo test
$ grcov ./target/coverage/raw/ --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html

```
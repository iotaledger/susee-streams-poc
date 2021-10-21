# Streams Author Tool

## About
This is a tiny test application providing a command line interface (CLI) to evaluate iota streams functionality for the
SUSEE project. All encrypted messages that are send or received to/from the tangle are loged to the console. 

## Prerequisites
To build this application, you need the following:
- [Rust](https://www.rust-lang.org/tools/install)
- (Optional) An IDE that supports Rust autocompletion. We recommend [Visual Studio Code](https://code.visualstudio.com/Download) with the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer) extension

We also recommend updating Rust to the [latest stable version](https://github.com/rust-lang/rustup.rs#keeping-rust-up-to-date):

```bash
rustup update stable
```

## Build

Build as usual using `build` or `run` with or without `--release` 
```bash
cargo build
```
The main.rs file does not support `no_std`. If cargo nightly is used you can use `no_std` to build all other files 
combined with your own `no_std` conform main.js.

## CLI API reference

Using the --help option of the build streams-author-tool will show following help text

```bash
target/release/streams-author-tool --help
```

            Test tool to evaluate iota streams functionality for the SUSEE project.
            
            USAGE:
                streams-author-tool [OPTIONS]
            
            OPTIONS:
                -f <FILES_TO_SEND>...        List of message files that will be encryped and send using the
                                             streams channel.
                                              [default: test/payloads/meter_reading-1-compact.json]
                -h, --help                   Print help information
                -n <NODE>                    The url of the iota node to connect to.
                                             Use 'https://chrysalis-nodes.iota.org' for the mainnet.
                                             
                                             As there are several testnets have a look at
                                                 https://wiki.iota.org/learn/networks/testnets
                                             for alternative testnet urls.
                                             
                                             Example:
                                                 The iota chrysalis devnet: https://api.lb-0.h.chrysalis-
                                             devnet.iota.cafe
                                              [default: https://chrysalis-nodes.iota.org]
                -V, --version                Print version information


## Examples

Without any cli arguments just using the default options as described in the help text:
```bash
target/release/streams-author-tool
```

Use the ota chrysalis devnet:
```bash
target/release/streams-author-tool -n https://api.lb-0.h.chrysalis-devnet.iota.cafe
```

Send two messages instead of the default message:
```bash
target/release/streams-author-tool -f "test/payloads/meter_reading-1.json" "test/payloads/meter_reading-1-compact.json"
```

# Streams Author Tool

## About
This is a tiny test application providing a command line interface (CLI) to evaluate iota streams functionality for the
SUSEE project. All encrypted messages that are send or received to/from the tangle are loged to the console. 

Channel specific aspects for the SUSEE project are:
* One single branch per sensor
* Sensor will be a subscriber and will be the only publishing actor in the single branch
* Energy provider will be the author
* Additional stakeholders (e.g. home owner) could be added as reading subscribers to the single branch
* Handshake:
  * The initial handshake (announcement/subscription/keyload) between sensor and the channel author will be done before
    a sensor is installed in a home, which means for the inital handshake the limitations of lorawan don't apply
  * If anything changes in the single branch channel setup, e.g. the addition of a new reading subscriber, the sensor
    will have to be able to receive new keyload information downstream via lorawan
    
The current implementation can only be used to evaluate encrypted streams package sizes for specified payload messages.
The above given aspects are currently not taken into account. The <a href="#todos">Todos Section</a> describes future
behaviour of this test tool in more detail. 

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

Use the iota chrysalis devnet:
```bash
target/release/streams-author-tool -n https://api.lb-0.h.chrysalis-devnet.iota.cafe
```

Send two messages instead of the default message:
```bash
target/release/streams-author-tool -f "test/payloads/meter_reading-1.json" "test/payloads/meter_reading-1-compact.json"
```

## Todos

### One application per workflow
Provide one application for each different workflow used for streams channel management and data transmission in the SUSEE project.
Following workflows will exist for each channel. Every sensor uses one single channel:

* Initialization<br>
  Limitations of lorawan don't apply.
  * Initial handshake (announcement/subscription/keyload) between sensor and the channel author.
  * Removing / Adding subscribers.
  
* Sensor Processing<br>
  `no_std` is needed.
  * Create encrypted streams packages for the smart meter messages to be send via lorawan.
    Lorawan is  simulated using binary files.
  * React to new keyload messages when the subscribers are removed or added.
  
* Application Server Processing<br>
  Limitations of lorawan don't apply.
  * Receive encrypted streams packages via lorawan (simulated using binary files) and send them to iota tangle.

Notes:
* Currently the author is the publisher. First step is to separate author and publisher roles.
* Applications for *Sensor Processing* and *Application Server Processing* can be run in two shells in parallel
  so that the applications can react to new lorawan messages (simulated using binary files).

### Use preshared keys for author and sensor
Currently, no pre shared keys are used.
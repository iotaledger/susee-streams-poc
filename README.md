# Streams Author Tool

## About
This project contains three tiny test applications providing command line interfaces (CLI) to evaluate the iota streams functionality for the
SUSEE project. All encrypted messages that are send or received to/from the tangle are loged to the console or written to binary files that are used to imitate a lorawan connection.

Following test applications are contained. For more details please see below in the <a href="#applications-and-workflows">Applications and workflows</a> section:
* *Sensor*<br>
  Imitates the processes running in the smart meter (a.k.a. *Sensor*)
 * *Management Console*<br>
  Imitates the processes needed for *Initialization* of the sensor, monitoring of *Sensor Processing* and managing the 
  *Add/Remove Subscriber* workflow
 * *Tangle Proxy*<br>
   Imitates processes in the Application Server and used by the initialization software performing the *Initialization* of the sensor.

The Channel used for the SUSEE project generally can be described as follows:
* One single branch per sensor
* Sensor will be a subscriber and will be the only publishing actor in the single branch
* Energy provider will be the author
* Additional stakeholders (e.g. home owner) could be added as reading subscribers to the single branch
* Handshake:
  * The initial handshake (announcement/subscription/keyload) between sensor and the channel author will be done before
    a sensor is installed in a home, which means for the inital handshake the limitations of lorawan don't apply
  * If anything changes in the single branch channel setup, e.g. the addition of a new reading subscriber, the sensor
    will have to be able to receive new keyload information downstream via lorawan
## Prerequisites
To build the applications, you need the following:
- [Rust](https://www.rust-lang.org/tools/install)
- (Optional) An IDE that supports Rust autocompletion. We recommend [Visual Studio Code](https://code.visualstudio.com/Download) with the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer) extension

We also recommend updating Rust to the [latest stable version](https://github.com/rust-lang/rustup.rs#keeping-rust-up-to-date):

```bash
rustup update stable
```

## Build

Build as usual using `build` or `run` with or without `--release`.

In the project root folder:
```bash
cargo build
```

Every application has its own crate so you might want to build only one application like this:

In the project root folder:
```bash
cargo build --package management-console  # alternatively 'sensor' or "tangle-proxy"
```

## CLI API reference

Using the --help option of the applications will show following help text

```bash
target/release/management-console --help # alternatively 'sensor' or "tangle-proxy"
```

            Management console for streams channels used in the SUSEE project

            USAGE:
                management-console [OPTIONS]

            OPTIONS:
                -c                                  Use this option to create (announce) the channel.
                                                    The announcement link will be logged to the console.
                -h, --help                          Print help information
                -k <SUBSCRIPTION_PUB_KEY>           Public key of the sensor subscriber.
                                                    Will be logged to console by the sensor app.
                -l <SUBSCRIPTION_LINK>              Subscription message link for the sensor subscriber.
                                                    Will be logged to console by the sensor app.
                -n <NODE_URL>                       The url of the iota node to connect to.
                                                    Use 'https://chrysalis-nodes.iota.org' for the mainnet.
                                                    
                                                    As there are several testnets have a look at
                                                        https://wiki.iota.org/learn/networks/testnets
                                                    for alternative testnet urls.
                                                    
                                                    Example:
                                                        The iota chrysalis devnet:
                                                        https://api.lb-0.h.chrysalis-devnet.iota.cafe
                                                    [default: https://chrysalis-nodes.iota.org]
                -s <SEED_FILE_PATH_AND_NAME>        Specifies the seed file to use.
                                                    Set this to path and name of the seed file.
                                                    If this option is not used:
                                                    * A file 'channel-seed.txt' is used if existing
                                                    * If 'channel-seed.txt' does not exist:
                                                      A new seed is created and written into a new file
                                                      'channel-seed.txt'.
                -V, --version                       Print version information
## Examples

******************************************************************************************
TODO ******************** Examples for the new applications ******************************
******************************************************************************************

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

## Applications and workflows 
### Applications
For each service being part in the different workflows a console application is provided to test the
streams channel management and data transmission in the SUSEE project. These services (or apps) are 
* *Sensor*<br>
  Running on the smart meter device
* *Management Console*<br>
  Running at the energy provider
* *Tangle Proxy*<br>
  Running in the application server or as part of the initialization software at the energy provider
 
Lorawan and other inter process communication is simulated using binary input and output files. The applications can be run in three shells in parallel so that the applications can react to new lorawan or tangle
messages. Each transfered 
package will be written into a separate file.

The services are characterized by following properties/aspects:

* *Sensor*
  * Online access:
    * *Initialization*: Wifi or wired using peripherals (e.g. usb)
    * *Sensor Processing*: Wireless via lorawan
    * *Add/Remove Subscriber*: Wireless via lorawan
  * Low processing capabilities<br>
    Following applies to all workflows (*Initialization*, *Sensor Processing*, *Add/Remove Subscriber*):
    Due to the low processing capabilities the sensor does not send the streams packages to the tangle directly but sends
    the packages to the *Tangle Proxy*. This way it does not need to process the adaptive POW.<br>
    Streams packages coming from the tangle are also received via the *Tangle Proxy*.
  * `no_std` is needed for Rust implementation. A specialized new delete operator may be needed for the C++ implementation.
    FreeRTOS will most probably be available (e.g. [ESP-IDF FreeRTOS](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-guides/freertos-smp.html)).

 * *Management Console*<br>
   Software needed for *Initialization* of the sensor, monitoring of *Sensor Processing* and managing the 
   *Add/Remove Subscriber* workflow. No Hardware or performance restrictions. 

 * *Tangle Proxy*
   * Is used in the 
     * Application Server for *Sensor Processing* and *Add/Remove Subscriber* workflows
     * Initialization software as part of the *Management Console* for the *Initialization* of the sensor
   * Fast online access
   * Connected to the *Sensor* via
     * lorawan for *Sensor Processing* and *Add/Remove Subscriber* workflows
     * Wifi or wired for the *Initialization* workflow
   * Receives prepared iota streams packages from the *Sensor* and sends these packages to the tangle performing the adaptive POW.
   * Listens to new tangle messages and sends the encrypted streams packages to the sensor:
     * Announcement Messages: Used in the *Initialization* workflow 
     * Keyload Messages: Used in the *Add/Remove Subscriber* and *Initialization* workflows              

Following workflows will exist for each channel. Every sensor uses its own exclusive channel:

#### Initialization
  * Limitations of lorawan don't apply. Sensor is connected via Wifi or wired using peripherals (e.g. usb).
  * Performs the initial handshake (announcement/subscription/keyload) between sensor and the channel author (*Management Console*)
    via the *Tangle Proxy*.
<img src="workflow_initialization.png" alt="drawing" width="650"/>

#### Add/Remove Subscriber
  Adding or removing subscribers from the channel. Here lorawan is also used for a back channel from application server
  to the *Sensor*.
<img src="workflow_add_remove_subscriber.png" alt="drawing" width="800"/>
  
#### Sensor Processing
  Smart meter messages are created and encrypted in streams packages by the *Sensor*. The packages are send via lorawan to the application server.
<img src="workflow_sensor_processing.png" alt="drawing" width="800" class="center"/>
  
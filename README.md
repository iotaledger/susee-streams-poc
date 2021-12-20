# Streams Author Tool

## About
This project contains three tiny test applications providing command line interfaces (CLI) to evaluate the iota streams functionality for the
SUSEE project.

Following test applications are contained. For more details please see below in the <a href="#applications-and-workflows">Applications and workflows</a> section:
* *Sensor*<br>
  Imitates the processes running in the smart meter (a.k.a. *Sensor*)<br>
  Can only be used together with a running *Tangle Proxy* instance
 * *Management Console*<br>
  Imitates the processes needed for *Initialization* of the sensor, monitoring of *Sensor Processing* and managing the 
  *Add/Remove Subscriber* workflow
 * *Tangle Proxy*<br>
   Imitates processes in the Application Server and used by the initialization software performing the *Initialization*
   of the sensor<br>
   Provides an http rest api used by the sensor to access the tangle

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

### Common CLI options and i/o files

Using the --help option of all three applications will show the app specific help text:
```bash
target/release/management-console --help # Use 'sensor' or "tangle-proxy" instead of 'management-console' for the other apps
```

*Management Console* and *Sensor* provide the following options. *Tangle-Proxy* uses the same options expect `--wallet-file`
as the *Tangle-Proxy* does not need a wallet:

    -h, --help
            Print help information

    -n, --node <NODE_URL>
            The url of the iota node to connect to.
            Use 'https://chrysalis-nodes.iota.org' for the mainnet.
            
            As there are several testnets have a look at
                https://wiki.iota.org/learn/networks/testnets
            for alternative testnet urls.
            
            Example:
                The iota chrysalis devnet:
                https://api.lb-0.h.chrysalis-devnet.iota.cafe
             [default: https://chrysalis-nodes.iota.org]

    -V, --version
            Print version information

    -w, --wallet-file <WALLET_FILE_PATH_AND_NAME>
            Specifies the wallet file to use.
            Set this to path and name of the wallet file.
            If this option is not used:
            * A file 'wallet-<APPLICATION-NAME>.txt' is used if existing
            * If 'wallet-<APPLICATION-NAME>.txt' does not exist:
              A new seed is created and written into a new file
              'wallet-<APPLICATION-NAME>.txt'.

*Management Console* and *Sensor* use the following files for persistence
* Wallet for the user seed<br>
  The applications are using a plain text wallet that stores the automatically generated seed in a text file.
  If option '--wallet-file' is not used a default filename 'wallet-<APPLICATION-NAME>.txt' is used.
  If the file does not exist a new seed is created and stored in a new wallet file. Otherwise the seed stored
  in the wallet file is used.<br>
  As the wallet file contains the plain text seed (not encrypted) make absolutely sure to<br>
  **DO NOT USE THIS WALLET FOR PRODUCTION PURPOSES**<br>
  Instead implement the [SimpleWallet trait](streams-tools/src/plain_text_wallet.rs)
  using a secure wallet library like [stronghold](https://github.com/iotaledger/stronghold.rs). 

* User state<br>
  On application start the current user state is loaded from a file named 'user-state-<APPLICATION-NAME>.bin'.
  On application exit the current user state is written into this file.

### Management Console CLI

    -c, --create-channel
            Use this option to create (announce) the channel.
            The announcement link will be logged to the console.

    -k, --subscription-pub-key <SUBSCRIPTION_PUB_KEY>
            Public key of the sensor subscriber.
            Will be logged to console by the sensor app.

    -l, --subscription-link <SUBSCRIPTION_LINK>
            Subscription message link for the sensor subscriber.
            Will be logged to console by the sensor app.


### Sensor CLI

    -s, --subscribe-announcement-link <SUBSCRIBE_ANNOUNCEMENT_LINK>
            Subscribe to the channel via the specified announcement link.
            
    -r, --register-keyload-msg <KEYLOAD_MSG_LINK>
            Register the specified keyload message so that it can be used
            as root of the branch used to send messages later on.

    -f, --file-to-send <FILE_TO_SEND>...
            A message file that will be encrypted and send using the streams channel.
            If needed you can use this option multiple times to specify several message files.

### Tangle Proxy CLI
Currently, the Tangle Proxy does not have any special CLI options except those described in the
<a href="#common-cli-options-and-io-files">Common CLI options section</a>.

Use the `--node` option to specify the url of the iota node to connect to.

## Example Workflow

### Sensor Initialization

**Create the channel using the *Management Console***

In the /target/debug or release folder:
```bash
    > ./management-console --create-channel
    >
    > [Management Console] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > [Management Console] A channel has been created with the following announcement link:
    >                      Announcement Link: c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:56bc12247881ff94606daff2
    >                           Tangle Index: 491e1459e1bc6200b741fdc90fac8058bacc9c37f6c56ed4d1ce38ef3493f13e
```
**Subscribe the *Sensor***

To use the sensor we need to start the *Tangle Proxy* first:
```bash
    > ./tangle-proxy
    > 
    > [Tangle Proxy] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > Listening on http://127.0.0.1:50000
```

Now the subscription message can be created using the announcement link from the console log of the *Management Console* above.<br>
In a second command shell in the /target/debug or release folder:
```bash
    > ./sensor --subscribe-announcement-link\
             "c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:56bc12247881ff94606daff2"
    > 
    > [Sensor] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > [HttpClient.recv_message] Receiving message with 151 bytes payload:
    > @c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:56bc12247881ff94606daff2[000000010400000000000000000000000000c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500e000001c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d195379850003cfa4c38fe060b13252e5b89cc00b893c127ea716a387e5b035e029fc2141e009c080e08b3d6214383ad27718581b60cf36935b5a3e23825d0ceb0afe6aa8c0d]->00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
    > 
    > [HttpClient.send_message] Sending message with 279 bytes payload:
    > @c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:aa5fc8814ca5a81c0dbf2b7e[00005001040000000134c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d195379850000000000000000056bc12247881ff94606daff2000000000000000000399dc641cec739093ef6f0ecbac881d5f80b049fe1e2d46bc84cb5aff505f66b0e00000156bc12247881ff94606daff2d874444633bd857b36bb47869a5b52e4ff212d21c4394aecb4db93a9fc2f29050352c3d6c34136614cd5003f43ef470bc8b65d8e908fa21a41d2dbeabfbb8aa9011692c14ff33a4d7bf71b0d77d352dc5ed93ff04660dfc3eda84ab9665586c517a12e5900aa39878aee3c259cecc464a647277d3e30ed937d5deea44aff1bb7161dcaf1006e3339ae9ead4b35a47d422fd5b0cf0a66f83852f9b593869f650e]->00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
    > 
    > [Sensor] A subscription with the following details has been created:
    >          Subscription Link:     c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:aa5fc8814ca5a81c0dbf2b7e
    >               Tangle Index:     744935bae3a2e42acf0c9b2bf89cb42ef09351b40f92e4791438049a54a2ef4d
    >          Subscriber public key: 399dc641cec739093ef6f0ecbac881d5f80b049fe1e2d46bc84cb5aff505f66b
```

The subscription link and public key then must be used with the management-console to accept the subscription
```bash
    > ./management-console\
      --subscription-link "c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:aa5fc8814ca5a81c0dbf2b7e"\
      --subscription-pub-key "399dc641cec739093ef6f0ecbac881d5f80b049fe1e2d46bc84cb5aff505f66b"
    >
    > [Management Console] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > [Management Console] A keyload message has been created with the following keyload link:
    >                      Keyload link: c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:dc4567247bbb6396057bfba9
    >                      Tangle Index: 4ec4d90ef85ef06fb32617a7730b6e8f21029d7a1f59820da538fe5b9c26f105
```

To finalize the subscription the keyload message link has to be registered by the sensor because it is the root message
of the branch used by the sensor to publish its messages.
```bash
    > ./sensor --register-keyload-msg "c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:dc4567247bbb6396057bfba9"
    > 
    > [Sensor] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > [SubscriberManager.subscribe()] - Replacing the old previous message link with new keyload message link
    >                                   Old previous message link: 00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
    >                                   Keyload message link: c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:dc4567247bbb6396057bfba9
    > 
    > [Sensor] Messages will be send in the branch defined by the following keyload message:
    >          Keyload  msg Link:     c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:dc4567247bbb6396057bfba9
    >               Tangle Index:     4ec4d90ef85ef06fb32617a7730b6e8f21029d7a1f59820da538fe5b9c26f105
    >          Subscriber public key: 399dc641cec739093ef6f0ecbac881d5f80b049fe1e2d46bc84cb5aff505f66b
```

**Send messages using the *Sensor***

Make sure that the *Tangle Proxy* is up and running in another shell. The folder `test/payloads` contains several message files that can be
send like this:
```bash
    > ./sensor --file-to-send "../../test/payloads/meter_reading-1-compact.json"
    > 
    > [Sensor] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > [Sensor] Message file '../../test/payloads/meter_reading-1-compact.json' contains 136 bytes payload
    > 
    > [HttpClient.recv_message] Receiving message with 298 bytes payload:
    > @c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:dc4567247bbb6396057bfba9[00001001040000000134c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d195379850000000000000000056bc12247881ff94606daff2000000000000000200c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500e00000156bc12247881ff94606daff2ab2ee4a2ef69cc77508565156273038e010181ad4173c18805a62e4ce1a1296a7a184456fc9c22ebc79e8c406057f0e30f39a433d81038d16f742b3b32853f40f0499a5474f011c56f36cff88e4911b8ed033819684a59e1c4ca3ac5e7116d269616ff89107be9a487d45e091ed647a33b8c65c4fd9ef46f678ffe4ebe6ef76eba233144b44a210926561c391d6991591f36d7926d3360f3021e7cb80c13a48fc42e1b0a7f97c03cf75559569d2eee8729260b]->00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
    > 
    > [HttpClient.send_message] Sending message with 354 bytes payload:
    > @c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:b387f1dcf73e24ff466c493c[00003001040000000134c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000dc4567247bbb6396057bfba9000000000000000300399dc641cec739093ef6f0ecbac881d5f80b049fe1e2d46bc84cb5aff505f66b0e000001dc4567247bbb6396057bfba9399dc641cec739093ef6f0ecbac881d5f80b049fe1e2d46bc84cb5aff505f66b009836483f6d4f669a143d31c440a51f558173b51945a957eab424178e3be8785fe4faca5ba1aa5a53c8d3cfda3c666326c4f82bf071fd1aa56bf8d3347034b19e04ebadef5dc07a1109b2d1aecd571a20a060c445f49886b9c13eb6056dc715182e344626270951acfade220a72a87e4c2b430ae0be9a9dd9c4c178f6c73c152962035aea461d3dc27df1e8b5afe2faf4d758293b20bc03032a2f387373157ceff3dc996de7b67db89910b0ac4f5081dd516be8c4fed7034b434281fe5eb9c09d8cb2bdac3a0dc52e8e07]->00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
    > 
    > [Sensor] Sent msg from file '../../test/payloads/meter_reading-1-compact.json': c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:b387f1dcf73e24ff466c493c, tangle index: b583f1f2c64c00af178892cb52e113ea340469efffdc5b934af5a75e49022d20
    > 
    > [Sensor] A subscription with the following details has already been created:
    >          Subscription Link:     c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:aa5fc8814ca5a81c0dbf2b7e
    >               Tangle Index:     744935bae3a2e42acf0c9b2bf89cb42ef09351b40f92e4791438049a54a2ef4d
    >          Subscriber public key: 399dc641cec739093ef6f0ecbac881d5f80b049fe1e2d46bc84cb5aff505f66b
    > 
    > [Sensor] The last previously used message link is: c67551dade4858b8d1e7ff099c8097e0feda9c8584489ccdbdd046d1953798500000000000000000:b387f1dcf73e24ff466c493c
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
 
Lorawan and other inter process communication is simulated using a socket connection. The applications can be run in
three shells in parallel so that the applications can react to new lorawan or tangle messages.

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
  
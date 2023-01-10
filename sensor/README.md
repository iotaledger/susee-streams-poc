# Sensor Resources

This folder contains several *Sensor* specific projects for X86/PC and ESP32-C3
platforms. Here is an overview about the contained sub folders:

* main-rust<br>
  A *Sensor* application for X86/PC written in Rust
* main-rust-esp-rs<br>
  A *Sensor* application for ESP32-C3 written in Rust using the Espressif IDF SDK
* main-rust-pio<br>
  A *Sensor* application for ESP32-C3 written in Rust using Platform IO (currently not working)
* sensor-lib<br>
  A Rust library containing shared sources for all *Sensor* applications and the
  *streams-poc-lib*
* streams-poc-lib<br>
  A static library providing C bindings for all functions needed in the SUSEE-Module
  and a test application written in C to test the library
  
## CLI of the Sensor Applications

There are three different Sensor applications for different purposes:

| Application  |  Platform | Purpose                       | Progr. Language |
|--------------|-----------|-------------------------------|-----------------|
| *streams-poc-lib* test application | ESP32  | Test the streams-poc-lib functionality using its C binding| C |
| *ESP32 Sensor*                     | ESP32  | Test the Rust code that builds the foundation of the streams-poc-lib without the limitations of a foreign function interface | Rust |
| *Sensor remote control*            | X86/PC | Test the Rust code of *ESP32 Sensor* on X86/PC | Rust |

In addition to the common CLI options described in the
[CLI API section of the main README file](../README.md#common-cli-options-and-io-files)
all Sensor applications provide CLI commands to manage the Streams usage:
 
    -s, --subscribe-announcement-link <SUBSCRIBE_ANNOUNCEMENT_LINK>
            Subscribe to the channel via the specified announcement link.
            
    -r, --register-keyload-msg <KEYLOAD_MSG_LINK>
            Register the specified keyload message so that it can be used
            as root of the branch used to send messages later on.

    -f, --file-to-send [<FILE_TO_SEND>...]
            A message file that will be encrypted and send using the streams channel.
            The message will be resend every 10 Seconds in an endless loop.
            Use CTRL-C to stop processing.
            
    -p, --println-subscriber-status
            Print information about the current client status of the sensor.
            In streams the sensor is a subscriber so that this client status is called subscriber
            status.
            
        --clear-client-state
            Deletes the current client status of the sensor so that
            all subscriptions get lost and the sensor can be used to subscribe to a new Streams
            channel.
            TODO: In future versions the seed will also be replaced by a new generated seed.
            TODO: -----------------------------
                  --------  WARNING  ---------- Currently there is no confirmation cli dialog
                  -----------------------------       use this option carefully!
                              
As both Sensor applications running on ESP32 do not provide an interactive terminal, the 
x86/PC Sensor application (a.k.a *Sensor remote control*) can be used to remote control the ESP32
applications. The x86/PC Sensor provides following CLI commands to manage the
remote control functionality:

    -t, --iota-bridge-url <IOTA_BRIDGE_URL>
            The url of the iota-bridge to connect to.
            Default value is http://localhost:50000
            
            Example: iota-bridge-url="http://192.168.47.11:50000"

    -c, --act-as-remote-control
            Use this argument to remotely control a running sensor application on
            an embedded device. For example this
            
              > ./sensor --subscribe-announcement-link "c67551dade.....6daff2"\
                         --act-as-remote-control
            
            will make the remote sensor subscribe the channel via the specified
            announcement-link. This sensor app instance communicates with the remote sensor
            app via the iota-bridge application. Please make sure that both sensor
            app instances have a working connection to the running iota-bridge.
            
            If sensor and iota-bridge run on the same machine they can communicate over the
            loopback IP address (localhost). This is not possible in case the sensor runs on an
            external device (embedded MCU). In this case the iota-bridge needs to listen to
            the ip address of the network interface (the ip address of the device that runs
            the iota-bridge) so that the embedded sensor can access the iota-bridge.
            Therefore in case you are using 'act-as-remote-control' you will also need to use
            the 'iota-bridge' option to connect to the iota-bridge.

The *streams-poc-lib* test application can only bee remote controlled if the streams channel has
not already been initialized (further details can be found in the
[streams-poc-lib README](../sensor/streams-poc-lib/README.md)).

The x86/PC Sensor application can also be used to act as a remote controlled Sensor or let's say
to mock (or imitate) an ESP32 Sensor application.
This is especially usefull to test the *IOTA Bridge* and the *Management Console*
without the need to run ESP32 Hardware. The CLI command to mock an ESP32 Sensor is:

    -m, --act-as-remote-controlled-sensor
            Imitate a remote sensor resp. an ESP32-Sensor awaiting remote control commands.
            ESP32-Sensor here means the 'sensor/main-rust-esp-rs' application or the
            test app of the streams-poc-lib in an initial Streams channel state.
            
            This command is used to test the iota-bridge and the management-console application
            in case there are not enough ESP32 devices available. The sensor application will
            periodically fetch and process commands from the iota-bridge.
            
            If the iota-bridge runs on the same machine as this application, they can
            communicate over the loopback IP address (localhost). In case the sensor
            iota-bridge listens to the ip address of the network interface (the ip
            address of the device that runs the iota-bridge) e.g. because some ESP32
            sensors are also used, you need to use the CLI argument '--iota-bridge-url'
            to specify this ip address.

The `--act-as-remote-controlled-sensor` argument is especially useful to automatically initialize the x86/PC Sensor
in interaction with the 
[*Management Console* (`--init-sensor` argument)](../management-console/README.md#automatic-sensor-initialization).
If you want to exit the *Sensor* application after the initialization has been finished you can use 
the `--exit-after-successful-initialization` argument:

    -e, --exit-after-successful-initialization
            If specified in combination with --act-as-remote-controlled-sensor the command poll loop
            will be stopped after a KEYLOAD_REGISTRATION confirmation has been send to confirm
            a successfully processed REGISTER_KEYLOAD_MESSAGE command.
            This argument is useful when the sensor app runs in automation scripts to allow the
            initialization of the Sensor and the Sensor app should exit after successful
            initialization.



The x86/PC Sensor application can also be used to test the `lorawan-rest/binary_request` endpoint of the *IOTA Bridge*
application. This is done using the `--use-lorawan-rest-api` argument:

    -l, --use-lorawan-rest-api
            If used the Sensor application will not call iota-bridge API functions directly
            but will use its lorawan-rest API instead.
            This way the Sensor application imitates the behavior of an ESP32-Sensor connected
            via LoRaWAN and a package transceiver connected to the LoRaWAN application server
            that hands over binary packages to the iota-bridge.

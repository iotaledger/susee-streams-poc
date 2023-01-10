# LoraWan AppServer Mockup Tool

This application is needed to test the *streams-poc-lib* for the *ESP32 Sensor*. 
The *streams-poc-lib* provides a test application in its main.c file (sensor/streams-poc-lib/main/main.c)
that can be used to test the library via a WIFI connection instead of a LoRaWAN connection.

The binary packages that are send via WIFI cannot be sent directly to the *IOTA-Bridge*. The packages need to
be posted to the *IOTA-Bridge* using the `lorawan-rest` API endpoints of the *IOTA-Bridge*.

This *LoraWan AppServer Mockup Tool* receives the binary packages from an *ESP32 Sensor* via a socket connection
and posts these packages to the *IOTA-Bridge* `lorawan-rest` API functions. The resulting response is transmitted
via the socket connection back to the *ESP32 Sensor*.

In a real world scenario a service running on the LoRaWAN Application Server (or tightly connected to it) would
post the binary packages received via LoRaWAN to the *IOTA-Bridge* via the `lorawan-rest` API endpoints.
Therefore this application is called *LoraWan AppServer Mockup Tool*.

## Prerequisites and Build
Please have a look at the [Prerequisites](../README.md#prerequisites)
and [Build](../README.md#build) section of the main README of this repository.

## LoraWan AppServer Mockup Tool CLI

Additionally to those commands described in the
[CLI API section of the main README file](../README.md#common-cli-options-and-io-files)
the *LoraWan AppServer Mockup Tool* provides these CLI commands:

    -b, --iota-bridge-url <IOTA_BRIDGE_URL>
            The url of the iota-bridge to connect to.
            Default value is http://localhost:50000
            Example: iota-bridge-url="http://192.168.47.11:50000" [default: http://localhost:50000]

    -l, --listener-ip-address <LISTENER_IP_ADDRESS_PORT>
            IP address and port to listen to.
            Example: listener-ip-address="192.168.47.11:50001"
            
            DO NOT USE THE SAME PORT FOR THE IOTA-BRIDGE AND THIS APPLICATION
             [default: localhost:50001]
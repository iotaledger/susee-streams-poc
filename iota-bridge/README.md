# IOTA Bridge

The *IOTA Bridge* is a REST service for messages transferred between the following list of actors:

* *Management Console*
* All kinds of *Sensor* applications like *ESP32 Sensor* and *X86/PC Sensor*
* *IOTA Tangle* nodes
* Services accessing the LoRaWAN AP-Server

It provides a REST API to:
* Send streams packages that will be attached to the tangle using an IOTA Node
  or receive existing streams packages from the tangle
* Send remote control commands from a *Sensor Remote Controll* or *Management Console*
  application to a *Sensor* application
* Send remote control confirmations from a *Sensor* application to the *Sensor Remote Controll*
  or *Management Console* application
* Receive IotaBridgeRequests containing one of the above described REST API requests as
  a binary serialized package which can be used to interact with the IOTA Bridge e.g. via LoRaWAN

## Prerequisites and Build
Please have a look at the [Prerequisites](../README.md#prerequisites)
and [Build](../README.md#build) section of the main README of this repository.

## IOTA-Bridge Console CLI

In addition to the common CLI options described in the
[CLI API section of the main README file](../README.md#common-cli-options-and-io-files)
the *IOTA-Bridge* offers the following CLI arguments.

    -l, --listener-ip-address <LISTENER_IP_ADDRESS_PORT>
            IP address and port to listen to.
            Example: listener-ip-address="192.168.47.11:50000"
            

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
            
## IOTA Bridge REST API
Most of the REST API is used internally by the accompanying susee-streams-poc applications. The only endpoint relevant
for public use is the IotaBridgeRequests API which can be called via the `lorawan-rest/binary_request` endpoints.

To demonstrate the usage of the API here is a cURL example:
```bash
    curl --location --request POST 'http://192.168.47.11:50000/lorawan-rest/binary_request?deveui=4711' \
         --header 'Content-Type: application/octet-stream' \
         --data-binary '@~/path-to-my-develop-folder/susee-streams-poc/test/iota-bridge/request_parts.bin'
```

**Underlying usecase:**<br>
Given you are using the streams-poc-lib function
[send_message()](../sensor/streams-poc-lib/components/streams-poc-lib/include/streams_poc_lib.h)
in your C code you will receive a binary package via the `lorawan_send_callback` function that you need
to specify to call send_message(). You'll transmit this binary package e.g. via LoRaWAN. In your LoRaWAN Application
Server you can use the `lorawan-rest/binary_request` endpoint of the *IOTA Bridge* to hand the binary package over to it. 

The body of the resulting HTTP Resonse needs to be returned to the *ESP32 Sensor* via the `response_callback`
function that is provided by the streams-poc-lib.

Have a look into the following documentation for more details:

* Interface of the streams-poc-lib: 
  [streams_poc_lib.h](../sensor/streams-poc-lib/components/streams-poc-lib/include/streams_poc_lib.h)
* Readme of the [streams-poc-lib](../sensor/streams-poc-lib/README.md)

The *LoraWan AppServer Mockup Tool* implements this process but uses a WIFI
socket connection instead of a LoRaWAN connection. For further details please
have a look into the
[*LoraWan AppServer Mockup Tool* README](lora-app-srv-mock/README.md).

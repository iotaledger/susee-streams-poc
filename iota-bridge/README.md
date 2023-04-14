# IOTA Bridge

The *IOTA Bridge* is a REST service for messages transferred between the following list of actors:

* *Management Console*
* All kinds of *Sensor* applications like *ESP32 Sensor* and *x86/PC Sensor*
* *IOTA Tangle* nodes
* Services accessing the LoRaWAN AP-Server

It provides a REST API to:
* Send streams packages that will be attached to the tangle using an IOTA Node
  or receive existing streams packages from the tangle
* Send remote control commands from a *x86/PC Sensor* or *Management Console*
  application to a *Sensor* application
* Send remote control confirmations from a *Sensor* application to the *x86/PC Sensor*
  or *Management Console* application
* Receive IotaBridgeRequest packages containing one of the above described REST API requests as
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

Most of the REST API is used internally by the accompanying susee-streams-poc applications.

The only endpoints relevant for public use are the following:

* <a href="#lorawan-rest-endpoints">/lorawan-rest</a> <br>
  Post binary IotaBridgeRequest packages received e.g. via LoRaWAN
* <a href="#lorawan-node-endpoints">/lorawan-node</a> <br>
  Manage LoRaWAN nodes (Sensors) cached by the *IOTA Bridge* to allow compressed Streams message usage

### lorawan-rest Endpoints

IotaBridgeRequest packages can be posted to the *IOTA Bridge* using the `lorawan-rest/binary_request` endpoints

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

The *AppServer Connector Mockup Tool* implements this process but uses a WIFI
socket connection instead of a LoRaWAN connection. For further details please
have a look into the
[*AppServer Connector Mockup Tool* README](../app-srv-connector-mock/README.md).

### lorawan-node Endpoints
To allow [compressed Streams message](../sensor/README.md#deveuis-and-compressed-streams-messages)
usage, the *IOTA Bridge* stores LoRaWAN nodes (a.k.a. Sensors in the SUSEE project)
in its <a href="#caching-of-lorawan-deveuis-and-streams-channel-meta-data">local SQLite3 database</a>.

The stored Sensors can be managed by the following API endpoints.

##### CREATE_NODE
Create a *Sensor* entry in the caching database.

    POST/lorawan-node/{devEui} ? channel-id = {channelId}

    devEui:     LoRaWAN DevEUI of the Sensor
    channelId:  Streams channel-id of the Sensor

Examples:<br>

* http://127.0.0.1:50000/lorawan-node/4711?channel-id=12345678 <br>
  Status 200 OK
  
##### GET_NODE
Query data of a specific Sensor

    GET /lorawan-node/{devEui}
    
    devEui: LoRaWAN DevEUI of the Sensor
    
Examples:<br>
Expecting that a Sensor with dev_eui 4711 is stored in the database.

* http://127.0.0.1:50000/lorawan-node/9876 <br>
  Status 404 Not Found<br>
  Body:
  
        Not Found
        Description: lorawan_node not found
  
* http://127.0.0.1:50000/lorawan-node/4711 <br>
  Status 200 OK<br>
  Body:
  
      {"dev_eui":"4711","streams_channel_id":"12345678"}
  
##### IS_NODE_KNOWN
Query if a specific Sensor is known by the *IOTA Bridge*

    GET /lorawan-node/{devEui} ? exist
    
    devEui: LoRaWAN DevEUI of the Sensor
    
Examples:<br>
Expecting that a Sensor with dev_eui 4711 is stored in the database.

* http://127.0.0.1:50000/lorawan-node/9876?exist <br>
  Status 404 Not Found<br>
  Body:
  
        Not Found
        Description: lorawan_node not found
  
* http://127.0.0.1:50000/lorawan-node/4711?exist <br>
  Status 200 OK<br>
  Body:
  
      1

## Caching of LoRaWAN DevEUIs and Streams Channel Meta Data

As been descibed in the
[Sensor README](../sensor/README.md#deveuis-and-compressed-streams-messages)
compressed streams messages can be used to
reduce the LoRaWAN payload size.
The usage of compressed messages is only possible after one or more normal streams messages have
been send. The *IOTA Bridge* then learns which Streams Channel ID is used
by which *Sensor* where the *Sensor* is identified by its 64 bit LoraWAN DevEUI.

The mapping of LoraWAN DevEUI to Streams Channel meta data is stored in a local SQLite3 database.
The database file "iota-bridge.sqlite3" is stored in the directory where the
*IOTA-Bridge* is started.

To review the data stored in the local SQLite3 database we recommend the
[DB Browser for SQLite](https://sqlitebrowser.org/) application.

## Use in Production

A network of LoRaWAN connected *Sensors* can consist of multiple millions of *Sensors*.
Given these *Sensors* would send messages every 15 minutes this leads to ~1.11 K request/s
per million users.

Despite all limitations that are caused by the available
[performance of the IOTA mainnet](https://blog.iota.org/chrysalis-iota-1-5-phase-1-now-live-on-mainnet-958ec4a4a415/),
this means that in a large scale scenario the *IOTA Bridge* would have to run in an industrial
web server tech stack including load-balancers, auto-scaling and so on.

In case [compressed streams messages](#caching-of-lorawan-deveuis-and-streams-channel-meta-data)
are used the *IOTA Bridge* needs an appropriately fast central or distributed
high availability data storage solution like (e.g. mariadb, postgres, mongodb, couchdb, ...).

Alternatively to handle thousands of requests per second using a high performance *IOTA Bridge*
the service could run on edge devices to handle only dozens or hundreds of requests in a specific
region resp. sector oft the LoRaWAN network. 



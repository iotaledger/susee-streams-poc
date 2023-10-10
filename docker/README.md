# Docker files for the SUSEE Streams POC Applications

This folder contains several docker related files to run the
susee-streams-poc applications with docker.
The included Dockerfile can be used to build docker images for the applications.
A docker-compose.yml file, located in the root folder of this repository,
specifies the needed connections to run a SUSEE POC system.

Although this README and the dockerfile are located in this folder,
**use the docker CLI only in the ROOT folder of this repository**.

## Using Docker Compose

The [docker-compose](../docker-compose.yml) file in the root folder of this repository,
defines a system with a running [iota-bridge](../iota-bridge/README.md), [management-console](../management-console/README.md),
[app-srv-connector-mock](../app-srv-connector-mock/README.md) and [x86/PC Sensor](../sensor/main-rust).
All relevant ports are published, so that the ports can be used via localhost or the external
ip address of the docker host machine.

To build all images and run the containers you just need:

```bash
  # in the root folder of the repository:
  > docker compose up
```

After the images have been build and the containers were started, the system is in a state as described in the
following sections.
The service names specified below are also used as target names in the [Dockerfile](#build-docker-images),
so that the service name will also be contained in all image names and can be used as tag
for `docker compose exec` or `docker compose run` calls. 

### IOTA Bridge
Service name: `iota-bridge`

State after compose up: The *IOTA Bridge* is running and can be accessed on the host machine via port 50000 or via
iota-bridge:50000 from within other docker containers.

### Management Console
Service name: `management-console`

State after compose up: The *Management Console* is started using the
[--run-explorer-api-server](../management-console/README.md#run-message-explorer)
argument to start the [message explorer](../test/README.md#view-sensor-messages-using-the-message-explorer).
The message explorer can be accessed on the host machine via port 8080 (for example: 127.0.0.1:8080).

To open the swagger-ui API documentation for the message explorer please open the following link after
the container has started:
http://127.0.0.1:8080/swagger-ui/#/nodes/nodes_index

Use `docker compose exec management-console ./management-console` followed by
[management-console CLI arguments](../management-console/README.md#management-console-cli)
to use the management console manually. See the
[Sensor Initialization](#sensor-initialization) section below for an example how to
use the management console for automatic sensor initialization.

### x86/PC Sensor
Service name: `sensor`

State after compose up: The *x86/PC Sensor* will try to send the message _meter_reading_1_compact.json_ file.
If the sensor has not been initialized before, an error message is printed:

        [StreamsTransportSocket.new_from_url()] Initializing instance with options:
        StreamsTransportSocketOptions:
             http_url: http://iota-bridge:50000,
             dev_eui:  9894513939058021305,
             use_lorawan_rest:  true
        
        [Sensor] Message file 'meter_reading_1_compact.json' contains 136 bytes payload
        
        thread 'main' panicked at '[SubscriberManager.send_signed_packet()] - Before sending messages you need to subscribe to a channel. Use subscribe() and register_keyload_msg() before using this function.', /app/streams-tools/src/user_manager/subscriber_manager.rs:203:13

The container will be stopped after the error message occurs.
Please follow the steps described in the following section to initialize the sensor and
restart the container after successful initialization to send messages.

##### Sensor Initialization:

After the system has been started with `docker compose up` and the sensor container has automatically
been stopped with the error message described above, execute the following statement in the root folder of this repository:
```bash
  # in the root folder of the repository:
  docker compose run sensor ./sensor --act-as-remote-controlled-sensor --iota-bridge-url "http://iota-bridge:50000" -e
```
The *x86/PC Sensor* will call the *IOTA Bridge* directly using the docker bridge network and
the sensor will start to pull messages:

    Received Command::NO_COMMAND    
    Received Command::NO_COMMAND    
    Received Command::NO_COMMAND
    Fetching next command in 3 secs

In a second shell please execute the following statement to start the automatic sensor initialization 
option of the *Management Console*:
```bash
  # in the root folder of the repository:
  docker compose exec management-console ./management-console --init-sensor --iota-bridge-url "http://iota-bridge:50000"
```
The console log of the *Management Console* will look like this:

    [Management Console] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    [Management Console] Initializing remote sensor
    [Management Console] A channel has been created with the following announcement link:
    Announcement Link: 01dc8b65fa2174cf9bf4e565a601cd49af206734f7b00de790f57de3650a72ef0000000000000000:56b8fc7d0cec202d5d165246
    Tangle Index: b1e3e72d96f4716905956f77a8dd235ab290232ce5cb7047d59f42b847178c39
    
    [Management Console] Using http://iota-bridge:50000 as iota-bridge url
    [Management Console] Sending subscribe_announcement_link command to remote sensor.
    
    Received Confirmation::NO_CONFIRMATION    
    Fetching next confirmation in 1 secs
    [Management Console] Received confirmation for successful Subscription from remote sensor.
    Initialization count is 0
    Creating keyload_message for
    subscription: 01dc8b65fa2174cf9bf4e565a601cd49af206734f7b00de790f57de3650a72ef0000000000000000:bbeb667e18768d663bd1b032
    public key: 426f5b1b43bf1f3334e1a79a8e45fdb96db814338f39054841dc1992d286782c
    
    [Management Console] A keyload message has been created with the following keyload link:
    Keyload link: 01dc8b65fa2174cf9bf4e565a601cd49af206734f7b00de790f57de3650a72ef0000000000000000:276a2ce1072f6639b8db683d
    Tangle Index: d3537d043e8350fabd564f60c77d37c6c7ce93cdbd65f911cbcb52bbdb45922a
    
    [Management Console] Sending register_keyload_msg command to remote sensor.
    
    Fetching next confirmation in 1 secs
    [Management Console] Received confirmation for successful KeyloadRegistration from remote sensor.
    =========> Sensor has been fully initialized <===========

After the successfull sensor initialization has been indicated by the *Management Console* on the console log with
`=> Sensor has been fully initialized <=`,
the sensor will exit processing because we used the `-e` resp. `--exit-after-successful-initialization` option
when we started the sensor container (see `docker compose run sensor` statement above).

The sensor initialization is now complete and the sensor will start sending messages after it has been restartet
as been described in the following section.

##### Send messages using the sensor
The sensor will start sending the message _"meter_reading_1_compact.json"_ automatically after container startup.

To restart the sensor service, execute the following statement
```bash
  # in the root folder of the repository:
  docker compose start sensor
```

The console log will look like this:

    [StreamsTransportSocket.new_from_url()] Initializing instance with options:
    StreamsTransportSocketOptions:
         http_url: http://iota-bridge:50000,
         dev_eui:  9894513939058021305,
         use_lorawan_rest:  true
    
    [Sensor] Message file 'meter_reading_1_compact.json' contains 136 bytes payload
    
    Sending message file meter_reading_1_compact.json
    
    [StreamsTransportSocket.recv_message] Receiving message with 298 bytes tangle-message-payload:
    0000100104000000013401dc8b65fa2174cf9bf4e565a601cd49af206734f7b00de790f57de3650a72ef000000000000000056b8fc7d0cec202d5d16524600000000000000020001dc8b65fa2174cf9bf4e565a601cd49af206734f7b00de790f57de3650a72ef0e00000156b8fc7d0cec202d5d1652462390d5d5ad04ee84c1292e0503dc656501017eaec054208f67963e00dd4d9cd8f9eb973cd500c701756bc976a276ead5137cd76db7a23cd99338c1424bab118b252ea2c80b1de584ef2e1a6daa4e1e59d01c1da085163d0107633731c6212d1e855dde1789ebbbf4f08fca1f1fdea3dfd1ef0b793a370174444b846d14913750f118f555d0db3c28be57b84cf631648e05d043a4d26ede741ba45ebbd2b00f0de9182768c5fe93bd6a40ba9fdae03f59517c0f
    
    [StreamsTransportSocket.send_message] Sending message with 354 bytes tangle-message-payload:
    0000300104000000013401dc8b65fa2174cf9bf4e565a601cd49af206734f7b00de790f57de3650a72ef0000000000000000276a2ce1072f6639b8db683d000000000000000300426f5b1b43bf1f3334e1a79a8e45fdb96db814338f39054841dc1992d286782c0e000001276a2ce1072f6639b8db683d426f5b1b43bf1f3334e1a79a8e45fdb96db814338f39054841dc1992d286782c000d44fc9d1a6b10d9cdcda7157e1e26ce1c8bc954e7dfe78aa87ebbbf4e1ba1ed0a990c38b4f16a6773095e5f84b767ab5c589619bf87229d5d3d3f342e998f8c29478f8ea417ac256c3f4e95c302e8418a9ed855f990a5f3014a7781b92dd39df936bb31367365ed8212da6dcff9838cb0eded2fad2b976bebf4cef88e1b416854e70c7d6e8ed466d643d248882effa4fe0c296a0ff13827c3f0216a480502f9a7c7ada1d85ee2409531cd5b96e6fa687c497646d282d9702bf456f4cfcb875866a69b2bfa77adfae307
    
    Previous message address now is 01dc8b65fa2174cf9bf4e565a601cd49af206734f7b00de790f57de3650a72ef0000000000000000:0d563a792cbd97cad4565739

You may want to use the [message explorer](http://127.0.0.1:8080/swagger-ui/#/nodes)
to view the channel of the streams channel of the sensor
and to view the sensor messages. This is described
[here](../test/README.md#view-sensor-messages-using-the-message-explorer)
in more detail.

**Send messages using the CLI manually**

Use `docker compose run` **when the sensor container has stopped**, to send one of the test messages
contained in the [test/payloads](../test/payloads) folder.

The test message files contained in the *test/payloads* folder have been copied to the app
folder in the sensor container image, so that they can be used with the
[--file-to-send CLI argument](../sensor/README.md#cli-of-the-sensor-applications)
by its filename without a preceding path.

Here is an example statement for the message file _meter_reading_1.json_.

```bash
  # in the root folder of the repository:
  docker compose run sensor ./sensor --use-lorawan-rest-api -f "meter_reading_1.json" --iota-bridge-url "http://iota-bridge:50000"
```

Please avoid using "docker compose exec sensor" or to run multple sensor containers sending messages in parallel,
as this will result in two concurrently running threads sending messages
wich will result in undefined behavior.

To stop the sensor container we need to find out the container name
by using `docker compose ps`, followed by `docker stop` to stop it:

List all currently running docker compose containers in a second command shell.
Unfortunately `docker compose ps` will list all running containers, but misses those
that have been startet with `docker compose run`, therefore we need the `--all` and `--filter`
arguments:
```bash
  # in the root folder of the repository:
  docker compose ps --all --filter status=running
  NAME                                         IMAGE                                      COMMAND                  SERVICE                  CREATED             STATUS              PORTS
  susee-streams-poc-app-srv-connector-mock-1   susee-streams-poc-app-srv-connector-mock   "./app-srv-connector…"   app-srv-connector-mock   54 minutes ago      Up 54 minutes       0.0.0.0:50001->50001/tcp, :::50001->50001/tcp
  susee-streams-poc-iota-bridge-1              susee-streams-poc-iota-bridge              "./iota-bridge -l 0.…"   iota-bridge              54 minutes ago      Up 54 minutes       0.0.0.0:50000->50000/tcp, :::50000->50000/tcp
  susee-streams-poc-management-console-1       susee-streams-poc-management-console       "./management-consol…"   management-console       54 minutes ago      Up 54 minutes       127.0.0.1:8080->8080/tcp
  susee-streams-poc-sensor-run-80c0a61e523a    susee-streams-poc-sensor                   "./sensor --use-lora…"   sensor                   20 seconds ago      Up 18 seconds       
  ```
Stop the container that has been started lately (susee-streams-poc-sensor-run-80c0a61e523a in this example).
Please search for a container name containing the term "_sensor-run_", copy the container name and
replace `susee-streams-poc-sensor-run-80c0a61e523a` in the statement below with the copied container name.
```bash
  # in the root folder of the repository:
  docker stop sensor susee-streams-poc-sensor-run-80c0a61e523a
```

### AppServer Connector Mockup Tool
Service name: `app-srv-connector-mock`

State after compose up: The *AppServer Connector Mockup Tool* is running and can be accessed on
the host machine via  port 50001 or via
app-srv-connector-mock:50001 from within other docker containers.
It will connect the iota-bridge via the docker bridge network.

The *AppServer Connector Mockup Tool* can be used together with the
[ESP32 Sensor](../sensor/main-rust-esp-rs)
as been described in
[Send messages using the Sensor](../test/README.md#send-messages---streams-poc-lib-test-application)
section, because the port 50001 can be accessed via the external address of the docker host.

Please note that the *AppServer Connector Mockup Tool* will not be used by the *x86/PC Sensor* because it is not compatible
with it.

## Build Docker Images
The 'Dockerfile' contained in this folder can be used to build docker images for the
[iota-bridge](../iota-bridge/README.md), [management-console](../management-console/README.md),
[x86/PC Sensor](../sensor/main-rust) and the [app-srv-connector-mock](../app-srv-connector-mock/README.md)
applications.

The 'Dockerfile' is a [multi-stage build](https://docs.docker.com/build/building/multi-stage/) that creates 
slim images only containing the resources for the specific application. Each application has its own
[build target](https://docs.docker.com/build/building/multi-stage/#stop-at-a-specific-build-stage).

Instead of building the docker images manually we recommend to use the docker-compose.yml file as been described
in the sections above.

If you want to build the image for a specific application yourself, this can be done like this.

```bash
  # In the root folder of the repository:
  > docker build --target iota-bridge --tag iota-bridge -f docker/Dockerfile .
```
Please replace the --target and --tag value ('iota-bridge' in the example above) with the application name of your choice
('iota-bridge', 'sensor', 'management-console' or 'app-srv-connector-mock'). If no target is specified the default image 'iota-bridge'
will be built.

If you want to build images and provide them on [Docker Hub](https://hub.docker.com/) just follow these steps:
```bash
  # In the root folder of the repository:
  > docker build --target iota-bridge --tag <docker-hub-account-name-goes-here>/iota-bridge -f docker/Dockerfile .
  > docker login
  > docker push <docker-hub-account-name-goes-here>/iota-bridge:latest
```


## Start IOTA Bridge and AppServer Connector Mockup Tool as public available service

The following installation steps have been tested with Ubuntu 22.04.

The docker-compose.yml file expects an iota node to be run on the same
host system and to be available via a static ip address or domain name.

Prepare the server host system:
```bash
  # In the home folder of a sudo privileged user on the server system 
  > sudo ufw allow 50000,50001,50002/tcp
  > mkdir susee-poc
  > cd susee-poc
```

Upload some resources needed for the installation process to the server host system
(please replace username `<USER>` and domain name `<SERVER_HOST>` with their actual values):
```bash
  # In the folder where this README.md is located (docker folder)
  > scp server-install-resources/* <USER>@<SERVER_HOST>:~/susee-poc
```

On the server host system, please edit the file `~/susee-poc/env.example`
using an editor of you choice and set the static ip address resp. domain name
value for the `NODE_HOST` variable.
```bash
  # In the susee-poc folder we created above 
  > nano env.example
  
  # After env.example has been stored
  # Te .env file will be created by the following script execution
  > sudo ./prepare_docker.sh
  
    # We are now ready to start the susee-poc services as in the background
  > docker compose up -d
```

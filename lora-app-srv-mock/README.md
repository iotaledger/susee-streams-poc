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
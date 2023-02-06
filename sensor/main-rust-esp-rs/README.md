# ESP32 Sensor Application

A *Sensor* application for ESP32-C3 written in Rust using
[esp-rs/esp-idf-sys](https://github.com/esp-rs/esp-idf-sys) and the
[Espressif IDF SDK](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/about.html)

## About

This Sensor application can be build and flashed to ESP32-C3 platforms using tools
developed and provided by the [esp-rs](https://github.com/esp-rs) community. Its main purpose
is to test Rust code used for the [streams-poc-lib](../streams-poc-lib) without the limitations
of a foreign function interface.

## Prerequisites

To build the *ESP32 Sensor* application for ESP32 platforms (currently only ESP32-C3 provided), you need the following:

* If you want to flash the *Sensor* app on an ESP32-C3 device you need to install the Espressif software development environment.
  This is not needed if you only want to build the *ESP32 Sensor* app into an ELF file that can be flashed later on. 
  The Rust based build process for the *ESP32 Sensor* app uses its own copy of the needed Espressif tools that is
  automatically downloaded.<br>
  To install the Espressif software development environment please follow the instructions given in the
  [Prerequisites section of the Streams POC library](../streams-poc-lib#prerequisites). 
* Make sure your installed python3 version is >= 3.8 and pip is already installed
  (`sudo apt install python3-pip`).
* Check that your rustc version is >= 1.58.0 (see [Rust install hints in the main README](../../README.md#for-x86pc)).
* Use the stock nightly Rust compiler:
```bash
    rustup install nightly
    rustup default nightly
    # For future daily/weekly updates
    rustup update
```
* We also need rust-src to install cargo-espflash in one of the next steps
```bash
    rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
```
* Install clang version >= 12
```bash
    sudo apt-get update
    sudo apt-get install clang-12 --install-suggests
```
* Install [Cargo-Espflash](https://github.com/esp-rs/espflash)
```bash
    sudo apt-get install libudev-dev
    sudo apt-get install pkg-config
    cargo install cargo-espflash
    cargo install espflash
```
* Install [ldproxy](https://github.com/esp-rs/embuild/tree/master/ldproxy)
```bash
    cargo install ldproxy
```

The fundamentals of these build Prerequisites are taken from the
[Rust on ESP32 STD demo app](https://github.com/ivmarkov/rust-esp32-std-demo) project by Ivan Markov.
If you want to build the *ESP32 Sensor* for other ESP32 devices than ESP32-C3 you can try to follow the
instructions there to build for Extensa core based MCUs (ESP32-C3 is a Risc-V core based MCU).

## Build

All build steps must be executed in the main folder of this project (where this README is located):

Before building we need to specify the WiFi SSID, the WiFi password and the url of the used *IOTA-Bridge* as
environment variables. These variables will be hard coded into the *ESP32 Sensor*.
Currently this is the only way to initiate a socket connection to the ESP32.
This also means that currently you need to compile the ESP32 *Sensor* app yourself to test it:
```bash
export SENSOR_MAIN_POC_WIFI_SSID="Susee Demo"
export SENSOR_MAIN_POC_WIFI_PASS=susee-rocks
export SENSOR_MAIN_POC_IOTA_BRIDGE_URL="http://192.168.0.100:50000"
```

If you have no ESP32-C3 device you can just start the build using cargo-espflash.
The ELF file will be created in the project folder.
```bash
cargo espflash save-image ESP32-C3 sensor-esp-rs.elf --release
```

If you have an ESP32-C3 device you can plug in the usb (or other serial bus) cable of your board
and start the build (with or without `--release`):
```bash
cargo espflash --monitor --partition-table="partitions.csv" --release
```
Given you already installed all needed drivers to access the serial port of your board, the port will be
detected automatically by cargo-espflash. After the application has been build and flashed the log output
of the *ESP32 Sensor* app is displayed on the console. This is controlled by the `--monitor` option used above. 

## CLI

The CLI documentation can be found in the [Sensor README](../#cli-of-the-sensor-applications)


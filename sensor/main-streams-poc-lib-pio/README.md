# PlatformIO Sensor test application for the precompiled streams-poc-lib 

## About
This project uses the [streams-poc-lib](../streams-poc-lib/) library and 
the `main/main.c` file of the *streams-poc-lib test application*
to build the test application using
[PlatformIO](https://platformio.org/).

The project has been created using the *"New Project"* wizard of the
[PlatformIO IDE](https://platformio.org/install/ide?install=vscode)
for [VsCode](https://code.visualstudio.com/) for the board
[esp32-c3-devkitm-1](https://docs.platformio.org/en/latest/boards/espressif32/esp32-c3-devkitm-1.html)
and the framework [espidf](https://docs.platformio.org/en/stable/frameworks/espidf.html)
followed by several
config file changes that are documented below.

This project uses the platform
[espressif32@5.3.0](https://github.com/platformio/platform-espressif32/releases/tag/v5.3.0)
which supports ESP-IDF v4.4.3 which is currently used by all ESP32 projects in
this repository.

## Needed config file changes for esp32-c3-devkitm-1 + espidf projects
If you want to use the streams-poc-lib
in your own PlatformIO based project, the following config file changes should be
a good starting point for your *streams-poc-lib* integration:

##### platformio.ini:

    platform = espressif32@5.3.0
    board_build.partitions = partitions_susee.csv
    board_build.flash_mode = dio
    upload_protocol = esptool
    upload_port = /dev/ttyUSB0
    upload_speed = 115200
    monitor_port = /dev/ttyUSB0
    monitor_speed = 115200

The build_flags configuration depends on the location of the prebuild *streams-poc-lib* files
(`streams-poc-lib.a` and `streams-poc-lib.h`).
For example if the *streams-poc-lib* is located
in the subfolder `lib/streams-poc-lib` the build_flags would look like this:

    build_flags = -Ilib/streams-poc-lib/include -Llib/streams-poc-lib -llibstreams_poc_lib.a

This configuration would expect the *streams-poc-lib* files to be located like this
in your *PlatformIO* project:

    | - my-project-folder
    |       |
    |       | - platformio.ini
    |       |
    |       | - lib
    |            |
    |            | - streams-poc-lib
    |                   |
    |                   | - include
    |                   |     |
    |                   |     | - streams_poc_lib.h
    |                   |
    |                   | - libstreams_poc_lib.a
    |                   | - libstreams_poc_lib.d


##### sdkconfig.esp32-c3-devkitm-1

    CONFIG_ESP_MAIN_TASK_STACK_SIZE=98303
    CONFIG_MAIN_TASK_STACK_SIZE=98303
    
    # CONFIG_FATFS_LFN_NONE is not set
    CONFIG_FATFS_LFN_HEAP=y
    CONFIG_FATFS_MAX_LFN=255
    CONFIG_FATFS_API_ENCODING_ANSI_OEM=y
    # CONFIG_FATFS_API_ENCODING_UTF_16 is not set
    # CONFIG_FATFS_API_ENCODING_UTF_8 is not set
    
    CONFIG_FREERTOS_HZ=1000


## Prerequisites and Build
Before this project can be build you need to build the *streams-poc-lib*.
Have a look into the [streams-poc-lib README](../streams-poc-lib/README.md#prerequisites)
for more details.

The `build_flags` configuration in the `platformio.ini` file of this project references the
built *streams-poc-lib* files directly so that no further copy actions are needed:
```ini
    build_flags =
        -I../streams-poc-lib/build/esp-idf/streams-poc-lib/target
        -L../streams-poc-lib/build/esp-idf/streams-poc-lib/target/riscv32imc-esp-espidf/release/
        -llibstreams_poc_lib.a
```

FYI: Here is the location of the used *streams-poc-lib* files that exist after the
*streams-poc-lib* has been build:
* ../streams-poc-lib/build/esp-idf/streams-poc-lib/target/streams_poc_lib.h
* ../streams-poc-lib/build/esp-idf/streams-poc-lib/target/riscv32imc-esp-espidf/release/libstreams_poc_lib.a
* ../streams-poc-lib/build/esp-idf/streams-poc-lib/target/riscv32imc-esp-espidf/release/libstreams_poc_lib.d

After the *streams-poc-lib* has been build this project can be build using the 
[PlatformIO Toolbar](https://docs.platformio.org/en/latest/integration/ide/vscode.html#ide-vscode-toolbar).
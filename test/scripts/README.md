# Scripts for automatic SUSEE Streams POC tests

This folder contains two Python3 scripts, and an .env file to perform a multi *Sensor* test on x86/PC
xUnix systems. In this test a number of Sensors are initialized using a single `management-console`
and the *Sensor* instances send messages periodically to test the `iota-bridge` under conditions,
similar to real world conditions.

These are the used files:
* .env<br>
  A [dotenv file](https://hexdocs.pm/dotenvy/dotenv-file-format.html) specifiying global test configuration
  variables.
* prepare_multi_sensor_test.py<br>
  Creates all folders, copies all applications into the right folder and initializes all Sensors.
* run_multi_sensor_test.py<br>
  Used to start the multi sensor test after it has been prepared with `prepare_multi_sensor_test.py`
  
## Prerequisites

1) Build the project with **release** profile as been described in the [main README file](../../README.md#for-x86pc-1).
   Though debug profile would also do, it would be very time-consuming, because the POW is very slow for debug builds.

2) Run an `iota-bridge` instance in a shell on your test machine.<br>
   In the `./target/release` folder:
```bash
    > ./iota-bridge
```

## Prepare and run the multi *Sensor* test

#### Step 1 
Open the `./test/scripts/.env` file and edit the `NUMBER_OF_SENSORS` value according to you needs.

#### Step 2
Open a shell in the `./test/scripts` folder and execute
```bash
    > python3 prepare_multi_sensor_test.py
```
The script creates a `workspace` folder containing a number of initialized *Sensors*.
Each *Sensor* is contained in its own `sensor_#` folder, where `#` denotes the index number
of the specific *Sensor*.
   
After the folders have been created and all application files have been copied into the right place
each *Sensor* is initialized using a `management-console` instance that is started in the `workspace` folder.
The *Sensor* instances are initialized concurrently using the `--init-multiple-sensors` argument of the
*Management Console*. After all *Sensors* have been initialized the *Management Console* will search for
further *Sensors* that are available for initialization in an endless loop. You need to close the terminal
that has been used to start the python3 process to kill all spawned sub processes after all *Sensors* have
been initialized.

The output log of the *Sensors* are written into `prepare_multi_sensor_test.log` files located in each of the
`sensor_#` folder.

The output log of the `management-console` is written into a `prepare_multi_sensor_test.log` file located in
the `workspace` folder.

To find out if all *Sensors* have been initialized, have a look into the
`prepare_multi_sensor_test.log` in the `workspace` folder. This log file will contain many
"DevEUI: ANY - Received Confirmation::NO_CONFIRMATION" entries like this:

    [2024-02-14T14:27:17Z INFO  streams_tools::remote::remote_sensor] DevEUI: ANY - Received Confirmation::NO_CONFIRMATION
    [2024-02-14T14:27:20Z INFO  streams_tools::remote::remote_sensor] DevEUI: ANY - Received Confirmation::NO_CONFIRMATION
    [2024-02-14T14:27:23Z INFO  streams_tools::remote::remote_sensor] DevEUI: ANY - Received Confirmation::NO_CONFIRMATION
    [2024-02-14T14:27:26Z INFO  streams_tools::remote::remote_sensor] DevEUI: ANY - Received Confirmation::NO_CONFIRMATION
    [2024-02-14T14:27:29Z INFO  streams_tools::remote::remote_sensor] DevEUI: ANY - Received Confirmation::NO_CONFIRMATION
    [2024-02-14T14:27:32Z INFO  streams_tools::remote::remote_sensor] DevEUI: ANY - Received Confirmation::NO_CONFIRMATION

There will occur several "DevEUI: ANY - Received Confirmation::NO_CONFIRMATION" messages during the initialization
process as the *Management Console* periodically tries to find new *Sensors* ready to be initialized. These
"DevEUI: ANY - Received Confirmation::NO_CONFIRMATION" messages will be surrounded by other log messages that result from
the initialization of other *Sensors*. If only "DevEUI: ANY - Received Confirmation::NO_CONFIRMATION" messages are logged
(as shown above) the initialization of all other *Sensors* has been finished.

#### Step 3
In the same shell execute
```bash
    > python3 run_multi_sensor_test.py
```
The script starts each *Sensor* using the `--file-to-send` argument so that all *Sensors* start sending messages
immediately. As each *Sensor* sends the message periodically the test will run until it is cancelled using `CTRL+C`.

The output log of the *Sensors* is written into `run_multi_sensor_test.log` files located in each of the
`sensor_#` folder.

The `management-console` is not needed for sending messages. Therefore, there is no log file for it.
# Scripts for automatic SUSEE Streams POC tests

This folder contains two Python3 scripts, and an .env file to perform a multi *Sensor* test on X86/PC
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

The output log of the *Sensors* are written into `prepare_multi_sensor_test.log` files located in each of the
`sensor_#` folder.

The output log of the `management-console` is written into a `prepare_multi_sensor_test.log` file located in
the `workspace` folder.


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
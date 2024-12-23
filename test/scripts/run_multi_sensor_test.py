import shutil
import os
import subprocess
import time
from dotenv import load_dotenv

load_dotenv()

number_of_sensors = int(os.getenv('NUMBER_OF_SENSORS'))
rust_target_folder = os.getenv('RUST_TARGET_FOLDER')
workspace_folder = os.getenv('WORKSPACE_FOLDER')
iota_bridge_url = os.getenv('IOTA_BRIDGE_URL')
failover_iota_bridge = os.getenv('FAILOVER_IOTA_BRIDGE_URL')

compiled_sensor_src_path = rust_target_folder + r"/sensor"
compiled_mng_console_src_path = rust_target_folder + r"/management-console"

for indx in range(0, number_of_sensors):
    print('Start Sensor in environment #' + str(indx))
    sensor_test_folder = workspace_folder + r"/sensor_" + str(indx)
    if indx < number_of_sensors - 1:
        subprocess.Popen("./sensor --random-msg-of-size 50 --use-lorawan-rest-api --iota-bridge-url " + iota_bridge_url + " --failover-iota-bridge-url " + failover_iota_bridge + " 2>>run_multi_sensor_test.log", cwd=sensor_test_folder, shell=True)
        time.sleep(1)
    else:
        subprocess.run("./sensor --random-msg-of-size 50 --use-lorawan-rest-api --iota-bridge-url " + iota_bridge_url + " --failover-iota-bridge-url " + failover_iota_bridge + " 2>run_multi_sensor_test.log", cwd=sensor_test_folder, shell=True)


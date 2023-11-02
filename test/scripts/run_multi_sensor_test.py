import shutil
import os
import subprocess
from dotenv import load_dotenv

load_dotenv()

number_of_sensors = int(os.getenv('NUMBER_OF_SENSORS'))
rust_target_folder = os.getenv('RUST_TARGET_FOLDER')
workspace_folder = os.getenv('WORKSPACE_FOLDER')
iota_bridge_url = os.getenv('IOTA_BRIDGE_URL')

compiled_sensor_src_path = rust_target_folder + r"/sensor"
compiled_mng_console_src_path = rust_target_folder + r"/management-console"

for indx in range(0, number_of_sensors):
    print('Start Sensor in environment #' + str(indx))
    sensor_test_folder = workspace_folder + r"/sensor_" + str(indx)
    if indx < number_of_sensors - 1:
        subprocess.Popen("./sensor --file-to-send \"../../../payloads/meter_reading_1_compact.json\" --use-lorawan-rest-api --iota-bridge-url " + iota_bridge_url + " 2>>run_multi_sensor_test.log", cwd=sensor_test_folder, shell=True)
    else:
        subprocess.run("./sensor --file-to-send \"../../../payloads/meter_reading_1_compact.json\" --use-lorawan-rest-api --iota-bridge-url " + iota_bridge_url + " 2>run_multi_sensor_test.log", cwd=sensor_test_folder, shell=True)


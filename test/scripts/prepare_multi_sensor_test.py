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
node_host = os.getenv('NODE_HOST')

compiled_sensor_src_path = rust_target_folder + r"/sensor"
compiled_mng_console_src_path = rust_target_folder + r"/management-console"

os.makedirs(os.path.dirname(workspace_folder + r"/"), exist_ok=True)
shutil.copy(compiled_mng_console_src_path, workspace_folder)

for indx in range(0, number_of_sensors):
    print('\n-------------------------------------------------------------------------------------------------------')
    print('--- Preparing Sensor environment #' + str(indx))
    print('-------------------------------------------------------------------------------------------------------')
    dst_folder = workspace_folder + r"/sensor_" + str(indx)
    dst_path =  dst_folder + "/sensor"
    os.makedirs(os.path.dirname(dst_path), exist_ok=True)
    shutil.copy(compiled_sensor_src_path, dst_path)
    print('--- Copied sensor ' + str(indx) + '.\n--- Starting sensor with act-as-remote-controlled-sensor argument.')
    print('-------------------------------------------------------------------------------------------------------')
    subprocess.Popen("./sensor --act-as-remote-controlled-sensor --exit-after-successful-initialization --iota-bridge-url " + iota_bridge_url + " 2>prepare_multi_sensor_test.log", cwd=dst_folder, shell=True)
    print('--- Prepared sensor environment for initialization ' + str(indx))

print('--- Starting ManagementConsole for a multi sensor initialization')
subprocess.Popen("./management-console --init-multiple-sensors --iota-bridge-url " + iota_bridge_url + " --node " + node_host + " 2>>prepare_multi_sensor_test.log", cwd=workspace_folder, shell=True)

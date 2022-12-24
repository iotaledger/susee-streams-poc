import shutil
import os
import subprocess
from dotenv import load_dotenv

load_dotenv()

number_of_sensors = int(os.getenv('NUMBER_OF_SENSORS'))
rust_target_folder = os.getenv('RUST_TARGET_FOLDER')
workspace_folder = os.getenv('WORKSPACE_FOLDER')

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
    print('--- Copied sensor ' + str(indx) + '.\n--- Starting sensor initialization.')
    print('-------------------------------------------------------------------------------------------------------')
    subprocess.Popen("./management-console --init-sensor --iota-bridge-url \"http://127.0.0.1:50000\" >> prepare_multi_sensor_test.log", cwd=workspace_folder, shell=True)
    subprocess.run("./sensor --act-as-remote-controlled-sensor --exit-after-successful-initialization > prepare_multi_sensor_test.log", cwd=dst_folder, shell=True)
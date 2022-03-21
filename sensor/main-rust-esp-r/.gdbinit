# https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-guides/jtag-debugging/using-debugger.html
target extended-remote :3333
set remote hardware-watchpoint-limit 2
mon reset halt
maint flush register-cache
thb app_main
continue
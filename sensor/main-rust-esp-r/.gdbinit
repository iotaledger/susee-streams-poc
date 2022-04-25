file /home/christof/Develop/chrisgitiota/susee-streams-poc/target/riscv32imc-esp-espidf/debug/main-rust-esp-r
target extended-remote :3333
set remote hardware-watchpoint-limit 2
mon reset halt
maint flush register-cache
thb app_main
continue
#!/bin/bash

if [ -d "hornet" ]
then
  echo "Directory 'hornet' already exists. Exiting process."
else
  mkdir hornet
  cd hornet
  curl -L -O "https://github.com/iotaledger/node-docker-setup/releases/download/v1.0.0-rc.16/node-docker-setup_stardust-v1.0.0-rc.16.tar.gz"
  tar -zxf node-docker-setup_stardust-v1.0.0-rc.16.tar.gz

  patch -t docker-compose.yml ../docker-compose.hornet.patch
  patch -t docker-compose-https.yml ../docker-compose-https.patch
  patch -t prepare_docker.sh ../prepare_docker.sh.patch

  echo "======================================================================="
  echo "=== Bootstrapping the Hornet Node environment needs sudo privileges ==="
  echo "===                Please enter your sudo password below            ==="
  echo "======================================================================="

  sudo ufw allow 15600,80,443,4000,5550,9030/tcp
  sudo ufw allow 14626,4000/udp









fi
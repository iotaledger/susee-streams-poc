#!/bin/bash
DIR_NAME="hornet"

if [ -d $DIR_NAME ]
then
  echo "Directory '"$DIR_NAME"' already exists. Exiting process."
else
  mkdir $DIR_NAME
  cd $DIR_NAME
  curl -L https://node-docker-setup.iota.org/iota | tar -zx

  patch -t docker-compose.yml ../docker-compose.hornet.patch
  patch -t docker-compose-https.yml ../docker-compose-https.patch
  patch -t prepare_docker.sh ../prepare_docker.sh.patch
  patch -t config.json ../config.json.patch

  echo "======================================================================="
  echo "=== Bootstrapping the Hornet Node environment needs sudo privileges ==="
  echo "===                Please enter your sudo password below            ==="
  echo "======================================================================="

  sudo ufw allow 15600,80,443,4000,5550,9030/tcp
  sudo ufw allow 14626,4000/udp
fi
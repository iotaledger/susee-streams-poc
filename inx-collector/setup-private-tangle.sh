#!/bin/bash

if [ -d "priv_tangle" ]
then
  echo "Directory 'priv_tangle' already exists. Exiting process."
else
  mkdir priv_tangle
  cd priv_tangle
  curl -L -O "https://github.com/iotaledger/hornet/releases/download/v2.0.0-rc.6/HORNET-2.0.0-rc.6-private_tangle.tar.gz"
  tar -zxf HORNET-2.0.0-rc.6-private_tangle.tar.gz

  patch -t docker-compose.yml ../docker-compose.patch

  echo "======================================================================="
  echo "=== Bootstrapping the Hornet Node environment needs sudo privileges ==="
  echo "===                Please enter your sudo password below            ==="
  echo "======================================================================="

  sudo ./bootstrap.sh build
fi
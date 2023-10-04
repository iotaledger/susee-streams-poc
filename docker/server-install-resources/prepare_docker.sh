#!/bin/bash

# Pull latest images
docker compose pull

cp env.example .env

# Prepare db directory
mkdir -p data
mkdir -p data/iota-bridge
mkdir -p data/management-console

if [[ "$OSTYPE" != "darwin"* ]]; then
  chown -R 65532:65532 data
fi
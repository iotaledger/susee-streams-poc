#!/bin/bash
curl --location --request GET 'http://127.0.0.1:50000/lorawan-node/4711?exist' \
--header 'Content-Type: text/plain' \
--data-binary '@'
#!/bin/bash
curl --location --request POST 'http://127.0.0.1:50000/lorawan-node/4711?channel-id=9f40a7bbfdb449f46769d0f6b05853d5f9684bb1cd75c0b1b12b73ab6a891be10000000000000000' \
--header 'Content-Type: text/plain' \
--data-binary '@'
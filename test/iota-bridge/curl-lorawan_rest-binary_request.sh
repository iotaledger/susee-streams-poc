#!/bin/bash
curl --location --request POST 'http://127.0.0.1:50000/lorawan-rest/binary_request?deveui=4711' \
--header 'Content-Type: application/octet-stream' \
--data-binary '@request_parts.bin' 

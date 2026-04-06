#!/bin/bash
regions=$(curl -s https://raw.githubusercontent.com/jito-foundation/bam-plugins/refs/heads/regions/data/testnet-regions.txt)
for region in $regions; do
    echo "$region.testnet.bam.jito.wtf"
    #nslookup "$region.testnet.bam.jito.wtf" 2>/dev/null | awk '/^Address/ && !/#/ {
    #    ip = $2
    #    if (ip !~ /^(10\.|172\.(1[6-9]|2[0-9]|3[01])\.|192\.168\.|127\.|169\.254\.)/) print ip ":5012"
    #}'
done

# BAM Maker Client
This repository contains an example implementation of the client for the BAM plugin
TPU Maker transactions. The code contained in here is meant to provide an example of how to send updates to all BAM Nodes with plugins enabled.

The Plugin TPU operates as a UDP server and will expect wire format Solana transactions
exactly as they are received on UDP TPUs.

We recommend sending price updates to all enabled regions in-order to avoid maintaining complex leader tracking logic.

## Examples
`cargo run --example send_wire_transaction`
`cargo run --example send_wire_transaction_async`

## Enabled Testnet Regions

| Domain      | PTPU Port   |
| ----------- | ----------- |
| frankfurt.testnet.bam.jito.wtf      | 5012       |

## Enabled Mainnet Regions

| Domain      | PTPU Port   |
| ----------- | ----------- |

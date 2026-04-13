# BAM Maker Client
This repository contains an example implementation of the client for the BAM plugin
TPU Maker transactions. The code contained in here is meant to provide an example of how to send updates to all BAM Nodes with plugins enabled.

The Plugin TPU operates as a UDP server and will expect wire format Solana transactions
exactly as they are received on UDP TPUs.

We recommend sending price updates to all enabled regions in-order to avoid maintaining complex leader tracking logic.

## Usage

Transactions are sent in an identical binary format to the standard TPU.

This assumes you have sent a configuration to Jito Labs. Configurations should include per-market:
- The public key of the signer you intend to use for sending transactions
- The public key of the writable account being updated for a particular market
- The instruction data offset and length of the seqno/nonce for the enrolled program

Transaction packets are expected to:
- Have only a single signer
- That signer signer is enrolled in the plugin
- The transaction packet can be associated with a specific market via account keys
- A nonce/seqno is extractable from the transaction packet
- A compute budget instruction specifying the price per compute unit to at least 20 lamports per compute unit
- CPI other than Compute Budget instructions will be rejected by the plugin TPU

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

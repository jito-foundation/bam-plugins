# MPP - Maker Prioritization Plugin
This repository contains resources for onboarding to the BAM MPP Plugin.

The Plugin TPU operates as a UDP server and will expect wire format Solana transactions exactly as they are received on UDP TPUs.

We recommend sending price updates to all enabled regions in-order to avoid maintaining complex leader tracking logic.

## Usage

Transactions are sent in an identical binary format to the standard TPU.

This assumes you have sent a configuration to Jito Labs. Configurations should include per-market:
- The public key of the signer you intend to use for sending transactions
- The public key of the writable account being updated for a particular market
- The instruction data offset and length of the seqno/nonce for the enrolled program

Transaction packets are expected to:
- Have only a single signer
- That signer is enrolled in the plugin
- The transaction packet can be associated with a specific market via account keys
- A nonce/seqno is extractable from the transaction packet
- A compute budget instruction specifying the price per compute unit to at least 20 lamports per compute unit
- Instructions programs other than the enrolled program and Compute Budget will be rejected by the plugin TPU
- Writes to accounts other than enrolled writable accounts will be rejected by the plugin TPU

## Recommendations
- Since Maker transactions are prioritized for top of the batch, the loaded accounts data size instruction is often unecessary when considering the minimum compute unit price of 20 lamports per compute unit
- To avoid complex leader tracking logic, we recommend sending price updates to all enabled regions

## Am I Enrolled? 

Query a specific BAM Node to see if your signer and program are enrolled.

```
curl -sS http://frankfurt.mainnet.bam.jito.wtf:9090/api/v1/mpp/config
```

## API Transactions Endpoint

Coming soon.

## Enabled Testnet Regions

| Domain      | PTPU Port   |
| ----------- | ----------- |
| frankfurt.testnet.bam.jito.wtf      | 5012       |

## Enabled Mainnet Regions

| Domain      | PTPU Port   |
| ----------- | ----------- |
| amsterdam.mainnet.bam.jito.wtf   | 5012       |
| dallas.mainnet.bam.jito.wtf      | 5012       |
| dublin.mainnet.bam.jito.wtf      | 5012       |
| frankfurt.mainnet.bam.jito.wtf   | 5012       |
| lax.mainnet.bam.jito.wtf         | 5012       |
| london.mainnet.bam.jito.wtf      | 5012       |
| nyc.mainnet.bam.jito.wtf         | 5012       |
| pittsburgh.mainnet.bam.jito.wtf  | 5012       |
| singapore.mainnet.bam.jito.wtf   | 5012       |
| siauliai.mainnet.bam.jito.wtf    | 5012       |
| slc.mainnet.bam.jito.wtf         | 5012       |
| tokyo.mainnet.bam.jito.wtf       | 5012       |

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
- Since Maker transactions are prioritized for top of the batch, the set loaded accounts data size instruction is often overkill when considering the minimum compute unit price of 20 lamports per compute unit and top of batch prioritization
- To avoid complex leader tracking logic, we recommend sending price updates to all enabled regions


## Am I Enrolled? 

Query a specific BAM Node to see if your signer and program are enrolled.

```
curl -sS http://frankfurt.mainnet.bam.jito.wtf:9090/api/v1/mpp/config
```

## Enabled Regions

| Domain      | PTPU Port   |
| ----------- | ----------- |
| dallas.testnet.bam.jito.wtf       | 5012       |
| frankfurt.testnet.bam.jito.wtf    | 5012       |
| ny.testnet.bam.jito.wtf           | 5012       |
| slc.testnet.bam.jito.wtf          | 5012       |

## Enabled Mainnet Regions

| Domain      | PTPU Port   |
| ----------- | ----------- |
| amsterdam.mainnet.bam.jito.wtf   | 5012       |
| dallas.mainnet.bam.jito.wtf      | 5012       |
| dublin.mainnet.bam.jito.wtf      | 5012       |
| frankfurt.mainnet.bam.jito.wtf   | 5012       |
| lax.mainnet.bam.jito.wtf         | 5012       |
| london.mainnet.bam.jito.wtf      | 5012       |
| ny.mainnet.bam.jito.wtf          | 5012       |
| pittsburgh.mainnet.bam.jito.wtf  | 5012       |
| singapore.mainnet.bam.jito.wtf   | 5012       |
| siauliai.mainnet.bam.jito.wtf    | 5012       |
| slc.mainnet.bam.jito.wtf         | 5012       |
| tokyo.mainnet.bam.jito.wtf       | 5012       |



## MPP Transaction Endpoint

This endpoint is designed to provide insight into the lifecycle of an MPP transaction within the BAM node. This endpoint returns transaction events exclusively for transactions that have passed MPP checks in the plugin TPU and is not a general transaction events data source.

**Important**: This endpoint is heavily rate-limited and is intended solely for MPP failed transaction debugging. The rate-limit is set to 10 requests over a 10 second window.

Query for events related to a specific transaction signature. Event types include:

- `received` - The transaction was received by the plugin TPU and passed packet-level / enrollment checks. Transactions will have a 'received' event only if the packet arrived at the BAM node with an upcoming or current connected leader slot.
- `forwarded` - The transaction was forwarded to the connected leader for potential inclusion in the block. If a transaction was not forwarded, it was likely received too late in the leader rotation.
- `commit_attempted` - The connected leader attempted to commit the transaction state. This event will include a reason for failure if the transaction was not committed.

TransactionEventResponse

```
struct TransactionEventResponse {
    pub timestamp: u64,
    pub signature: String,
    pub slot: u64,
    // Event types include: received, forwarded, commit_attempted
    pub event_type: String,
    // Populated on received and commit_attempted events only.
    pub event_status: String,
    // Populated on 'received' events only. 'tpu' or 'ptpu'
    pub entrypoint: String,
}
```

Example query and response:

```
curl -sS "https://explorer.bam.dev/api/v1/mpp/transaction?txSignature=8Y99GGBg8u4nqLuVSdyCiooc1vGJtwXEd2MQxmAnj9u2GaSznoDxhYuH9m5z1P4UTrb6YGNjpFFGHNkLwUvJvtn" | jq .
[
  {
    "timestamp": 1777396897285,
    "signature": "8Y99GGBg8u4nqLuVSdyCiooc1vGJtwXEd2MQxmAnj9u2GaSznoDxhYuH9m5z1P4UTrb6YGNjpFFGHNkLwUvJvtn",
    "slot": 416263413,
    "event_type": "received",
    "event_status": "Ok(())",
    "entrypoint": "ptpu"
  },
  {
    "timestamp": 1777396897309,
    "signature": "8Y99GGBg8u4nqLuVSdyCiooc1vGJtwXEd2MQxmAnj9u2GaSznoDxhYuH9m5z1P4UTrb6YGNjpFFGHNkLwUvJvtn",
    "slot": 416263413,
    "event_type": "forwarded",
    "event_status": "",
    "entrypoint": ""
  },
  {
    "timestamp": 1777396897320,
    "signature": "8Y99GGBg8u4nqLuVSdyCiooc1vGJtwXEd2MQxmAnj9u2GaSznoDxhYuH9m5z1P4UTrb6YGNjpFFGHNkLwUvJvtn",
    "slot": 416263413,
    "event_type": "commit_attempted",
    "event_status": "COMMITTED",
    "entrypoint": ""
  }
]
```

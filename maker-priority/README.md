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
- To ensure your transactions are being sent to the proper BAM Node host in the event of a DNS failover, periodically poll for enabled BAM Node via DNS or derive MPP . See `Enabled Mainnet Regions`


## Am I Enrolled? 

Query a specific BAM Node to see if your signer and program are enrolled.

```
curl -sS http://frankfurt.mainnet.bam.jito.wtf:9090/api/v1/mpp/config
```

## What Validators on the Leader Schedule are Connected to BAM Nodes?

```
curl -sS https://explorer.bam.dev/api/v1/validators
```


## What Nodes are Connected to a Validator?

```
curl -sS http://frankfurt.mainnet.bam.jito.wtf:9090/api/v1/validators
```

## MPP Simulate Endpoint

Use this endpoint to check whether a transaction passes the MPP packet validation checks performed by a BAM node before signature verification.

Despite its name, this endpoint does not execute the transaction or simulate it against a bank. It also does not verify transaction signatures or guarantee that the transaction will be scheduled or land.

```http
POST /api/v1/mpp/simulate
Content-Type: application/json
```

#### Request Body

| Field         | Type   | Required | Description                                      |
|---------------|--------|----------|--------------------------------------------------|
| `transaction` | string | Yes      | Base64-encoded wire-format `VersionedTransaction`. |

The request body is limited to 2 KiB, and the decoded transaction must fit within Solana's packet size limit.

**Example request**
```bash
curl -sS -X POST "http://fra.mainnet.bam.jito.wtf:9090/api/v1/mpp/simulate" \
  -H "Content-Type: application/json" \
  -d '{"transaction":"BASE64_ENCODED_VERSIONED_TRANSACTION"}' | jq .
```

#### Responses

A transaction that passes MPP validation returns HTTP `200` with its extracted sequence number:

```json
{
  "status": "valid",
  "seqno": "42"
}
```

A transaction rejected by MPP validation returns HTTP `200` with the rejection reason:

```json
{
  "status": "invalid",
  "error": "signer is not enrolled"
}
```

Malformed base64, an invalid serialized transaction, or a decoded transaction over the packet size limit returns HTTP `400` with `status` set to `invalid` and a description in `error`.

## Enabled Regions

| Domain      | PTPU Port   |
| ----------- | ----------- |
| dfw.testnet.bam.jito.wtf  | 5012       |
| fra.testnet.bam.jito.wtf  | 5012       |
| ewr.testnet.bam.jito.wtf  | 5012       |
| slc.testnet.bam.jito.wtf  | 5012       |

## Enabled Mainnet Regions

**IMPORTANT**: Derive UDP destinations from gossip contact info or periodically poll for these DNS records to ensure your transactions are being sent to the proper BAM Node host in the event of a DNS failover. These records have a TTL of 300 seconds.


| Domain      | PTPU Port   |
| ----------- | ----------- |
| ams.mainnet.bam.jito.wtf  | 5012       |
| dfw.mainnet.bam.jito.wtf  | 5012       |
| dub.mainnet.bam.jito.wtf  | 5012       |
| fra.mainnet.bam.jito.wtf  | 5012       |
| iad.mainnet.bam.jito.wtf  | 5012       |
| lax.mainnet.bam.jito.wtf  | 5012       |
| lon.mainnet.bam.jito.wtf  | 5012       |
| ewr.mainnet.bam.jito.wtf  | 5012       |
| pit.mainnet.bam.jito.wtf  | 5012       |
| sin.mainnet.bam.jito.wtf  | 5012       |
| sea.mainnet.bam.jito.wtf  | 5012       |
| sqq.mainnet.bam.jito.wtf  | 5012       |
| slc.mainnet.bam.jito.wtf  | 5012       |
| tyo.mainnet.bam.jito.wtf  | 5012       |



## MPP Transaction Endpoint

This endpoint is designed to provide insight into the lifecycle of an MPP transaction within the BAM node. This endpoint returns transaction events exclusively for transactions that have passed MPP checks in the plugin TPU and is not a general transaction events data source.

**Important**: This endpoint is heavily rate-limited and is intended solely for MPP failed transaction debugging. The rate-limit is set to 250 requests over 10 seconds.

Query for events related to a specific transaction signature. Event types include:

- `received` - The transaction was received by the plugin TPU and passed packet-level / enrollment checks. Transactions will have a 'received' event only if the packet arrived at the BAM node with an upcoming or current connected leader slot.
- `forwarded` - The transaction was forwarded to the connected leader for potential inclusion in the block. If a transaction was not forwarded, it was likely received too late in the leader rotation.
- `commit_attempted` - The connected leader attempted to commit the transaction state. This event will include a reason for failure if the transaction was not committed.


```http
GET /api/v1/mpp/transaction?txSignature={txSignature}
```

#### Query Parameters

| Parameter     | Type   | Required | Description                                           |
|---------------|--------|----------|-------------------------------------------------------|
| `txSignature` | string | Yes      | The base58 encoded transaction signature to query.    |

#### Response Schema

**TransactionEventResponse**
```rust
struct TransactionResponse {
    pub events: Vec<TransactionEvent>,
}

struct TransactionEvent {
    pub timestamp: u64,
    pub signature: String,
    pub slot: u64,
    // Event types include: received, forwarded, commit_attempted
    pub event_type: String,
    // Populated on received and commit_attempted events only.
    pub event_status: String,
    // Populated on 'received' events only. 'tpu' or 'ptpu'
    pub entrypoint: String,
    // Unique identifier for the batch in which the transaction was scheduled
    pub batch_uuid: Option<String>,
}
```

**Commit Attempt Statuses**
- `COMMITTED`
- `ALREADY_PROCESSED`
- `INVALID_ACCOUNT_FOR_FEE`
- `ACCOUNT_NOT_FOUND`
- `OUTSIDE_LEADER_SLOT`
- `BLOCKHASH_NOT_FOUND`
- `POH_TIMEOUT`
- `WOULD_EXCEED_MAX_ACCOUNT_COST_LIMIT`
- `MAX_LOADED_ACCOUNTS_DATA_SIZE_EXCEEDED`

**Example query and response**
```bash
curl -sS "https://explorer.bam.dev/api/v1/mpp/transaction?txSignature=8Y99GGBg8u4nqLuVSdyCiooc1vGJtwXEd2MQxmAnj9u2GaSznoDxhYuH9m5z1P4UTrb6YGNjpFFGHNkLwUvJvtn" | jq .
[
  {
    "timestamp": 1777396897285,
    "signature": "8Y99GGBg8u4nqLuVSdyCiooc1vGJtwXEd2MQxmAnj9u2GaSznoDxhYuH9m5z1P4UTrb6YGNjpFFGHNkLwUvJvtn",
    "slot": 416263413,
    "event_type": "received",
    "event_status": "Ok(())",
    "entrypoint": "ptpu",
    "batch_uuid": "96c3c20a-12d6-4059-b17b-742334fad3c6"
  },
  {
    "timestamp": 1777396897309,
    "signature": "8Y99GGBg8u4nqLuVSdyCiooc1vGJtwXEd2MQxmAnj9u2GaSznoDxhYuH9m5z1P4UTrb6YGNjpFFGHNkLwUvJvtn",
    "slot": 416263413,
    "event_type": "forwarded",
    "event_status": "",
    "entrypoint": "",
    "batch_uuid": "96c3c20a-12d6-4059-b17b-742334fad3c6"
  },
  {
    "timestamp": 1777396897320,
    "signature": "8Y99GGBg8u4nqLuVSdyCiooc1vGJtwXEd2MQxmAnj9u2GaSznoDxhYuH9m5z1P4UTrb6YGNjpFFGHNkLwUvJvtn",
    "slot": 416263413,
    "event_type": "commit_attempted",
    "event_status": "COMMITTED",
    "entrypoint": "",
    "batch_uuid": "96c3c20a-12d6-4059-b17b-742334fad3c6"
  }
]
```

## MPP Batch Endpoint

This endpoint returns timing information for a specific BAM batch auction within a slot. Use the `batch_uuid` from the transaction endpoint to look up the batch window for transactions scheduled in that batch.

```http
GET /api/v1/mpp/batch/{slot}/{batch}
```

#### Path Parameters

| Parameter | Type   | Required | Description                                      |
|-----------|--------|----------|--------------------------------------------------|
| `slot`    | u64    | Yes      | The leader slot containing the MPP batch.        |
| `batch`   | string | Yes      | The UUID of the MPP batch to query.              |

#### Response Schema

**BatchResponse**
```rust
struct BatchResponse {
    pub slot: u64,
    pub batch_uuid: String,
    // Unix timestamp in nanoseconds when the batch opened.
    pub batch_start_time: u64,
    // Unix timestamp in nanoseconds when the batch closed.
    pub batch_end_time: u64,
}
```

**Example query and response**
```bash
curl -sS "https://explorer.bam.dev/api/v1/mpp/batch/425651119/1ef30c93-2a2c-466d-9f63-210d81a9446d" | jq .
[
  {
    "slot": 425651119,
    "batch_uuid": "1ef30c93-2a2c-466d-9f63-210d81a9446d",
    "batch_start_time": 1781136355462961432,
    "batch_end_time": 1781136355473540361
  }
]
```

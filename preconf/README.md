# BAM Preconfirmations

BAM Preconfirmations are a stream of the transactions a BAM-connected leader has
**committed to executing**, published before those transactions are shredded and
propagated across the network.

This folder holds the public protobuf definition of the stream. For the full
documentation, see [bam.dev — Preconfirmations](https://bam.dev/docs/bam/preconfirmations/).

## How to get access

The stream is distributed through Jito's partners, who handle pricing, onboarding,
connection details and support. Jito does not sell the stream directly, and you do not
need an account with Jito.

- **Helius:** [pre-confirmations documentation](https://www.helius.dev/docs/pre-confirmations/overview)
- **Triton:** [customer onboarding](https://customers.triton.one/onboarding/)

## What a preconfirmation is

When a BAM Node schedules a batch, it sends the batch to the leader. The leader commits to
executing it and confirms back to the node, and that confirmation triggers publication of
the batch's transactions on the stream.

- **A commitment, not a prediction.** Pending work that has not been scheduled and
  committed is never streamed.
- **Not finality.** The leader can still skip its slot, or the block can land on a fork.
  A preconfirmation says what will execute *if* the block lands.
- **BAM leaders only.** Non-BAM leaders, and BAM validators that have opted out, produce no
  preconfirmations.

## Files

| File | Contents |
| --- | --- |
| [`preconf.proto`](preconf.proto) | The `PreconfStream` gRPC service. **This is the current definition.** |
| [`preconf-auth.proto`](preconf-auth.proto) | An earlier copy of the same service, without `execution_success`. Kept for reference; use `preconf.proto`. |

## Wire format

`PreconfStream` is a server-streaming gRPC service:

```protobuf
service PreconfStream {
  rpc Subscribe(PreconfSubscribeRequest) returns (stream PreconfUpdate);
}
```

### `PreconfUpdate`

Each update carries one committed transaction.

| Field | Type | Meaning |
| --- | --- | --- |
| `slot` | `uint64` | The leader slot the transaction is scheduled for |
| `sequence_id` | `uint32` | The batch it was scheduled in |
| `bundle_position` | `uint32` | Its position within that batch |
| `is_revert_on_error` | `bool` | `true` if the batch is all-or-nothing (a bundle) |
| `transaction` | `bytes` | The serialized transaction |
| `execution_success` | `optional bool` | Whether the transaction executed successfully, once the validator has reported it. Absent when the outcome is not yet known. |

### `PreconfSubscribeRequest` (authentication)

Subscribers authenticate with an allowlisted Ed25519 keypair. Access is granted through the
distribution partners above.

1. Send your public key, base58-encoded, in the `x-bam-pubkey` gRPC metadata header.
2. Set `slot` to a recent slot of your choosing. It must be no more than 64 slots behind the
   node's current slot, and not in the future.
3. Set `signature` to the Ed25519 signature of that slot, encoded as 8 bytes little-endian.

A key that is not allowlisted is rejected with `PERMISSION_DENIED`. A missing key, a stale
or future slot, or a bad signature is rejected with `UNAUTHENTICATED`. A subscriber that
cannot keep up with the stream has updates dropped, so read it promptly.

## Learn more

- [Preconfirmations for searchers](https://bam.dev/docs/bam/preconfirmations/searchers/)
- [Preconfirmations for validators](https://bam.dev/docs/bam/preconfirmations/validators/)
- [BAM preconfirmations are live](https://bam.dev/blog/bam-preconfirmations-are-live/) (launch announcement)

# Multi-Market Update Coalesce

## What is a multi-market update? What benefits does it provide?

A multi-market update is an instruction that updates multiple writable accounts at a time. Multi-market updates allow multiple markets to be updated for a single signature base fee. Additionally, a signer can send multiple disjoint sets of updates through-out a batch and expect that only the relevant updates will be scheduled.

## Sequence Number

It is recommended to use an off-chain sequence number that increases over time or per-update to ensure that your program takes full advantage of sequence number based update coalescing.

## Coalesce Strategy

For a set of transactions received within a batch auction, the scheduler will only include a transaction if that transaction contains the highest seen sequence number for any writable account within the account keys. Highest seen sequence number will reset when the BAM node has no more connected leader slots to serve. Included transactions will be scheduled by order of ascencing sequence number.

**Examples**

| Sequence Number | Account Keys (w) | Transaction Scheduled |
|-----------------|------------------|-----------------------|
| 1               | A                | No                    |
| 2               | B, C             | Yes                   |
| 3               | A, B             | Yes                   |
In this example we drop sequence number 1 because there was a higher observed sequence number for every account included.

| Sequence Number | Account Keys (w) | Transaction Scheduled |
|-----------------|------------------|-----------------------|
| 1               | A                | Yes                   |
| 2               | B, C             | No                    |
| 3               | B, C, D          | No                    |
| 4               | B, C, D, E       | Yes                   |
In this example we drop sequence numbers 2 and 3 because sequence number 4 contains the highest sequence number for all included account keys.

| Sequence Number | Account Keys (w) | Transaction Scheduled |
|-----------------|------------------|-----------------------|
| 1               | A, B, C          | Yes                   |
| 2               | C, D, E          | Yes                   |
| 3               | E, F             | Yes                   |
In this example we schedule all transactions because each transaction contains the highest sequence number for at least one account key.

### Example Transactions

Mainnet program `CkMgJUnsXB85LLoQWznHAtyYtm2eAyVCiTbsxURKvyTt` has been used to validate multi-market update behavior for customers. The following transactions were scheduled with the following transaction flow to the plugin TPU:

The transactions below were received during slot `421288214` 's fourth batch auction.


| Sequence Number | Account Keys | Mainnet Transaction Signature |
|-----------------|--------------|-------------------------------|
| 67585 | `3xVyE2smd5aVA967PXUg1pvVqYoaPuqXjKkUDZNxQWkt` | `5CCn8T9TMPGjyvKjS7pzWMCRLF6hADG9fuyrvBJWJfE3Lxr4b251XKEQHwRthZqppBGZZVqyJLiyu8jCCcNaRUcW` |
| 67783 | `A8etPmrQwkhc32zvAhnCtXBEEUjK2N1A3qpei6VL26WS` | `N/A` |
| 67984 | `A8etPmrQwkhc32zvAhnCtXBEEUjK2N1A3qpei6VL26WS`<br>`BBBhtpBDczYRBKEG8bMC9MH1RCwrndPBzynvgdvaJF9a` | `N/A` |
| 68108 | `A8etPmrQwkhc32zvAhnCtXBEEUjK2N1A3qpei6VL26WS`<br>`BBBhtpBDczYRBKEG8bMC9MH1RCwrndPBzynvgdvaJF9a`<br>`FFbnv9SXM8on3MfmGLFZqyJhjh2Ns9TLEUgMiLe7BHkd` | `N/A` |
| 68353 | `A8etPmrQwkhc32zvAhnCtXBEEUjK2N1A3qpei6VL26WS`<br>`BBBhtpBDczYRBKEG8bMC9MH1RCwrndPBzynvgdvaJF9a`<br>`FFbnv9SXM8on3MfmGLFZqyJhjh2Ns9TLEUgMiLe7BHkd`<br>`C36UNg5ZbbB8z7ZTBXQvNJFDZu5uiLmDxJ73FnfBENUk` | `64QVNuSzUqCf8SEVJ8NaXPsFTiJ6DZSS7Nrv47dpjnfUsWjpJ55F1bKWjp5S4nDrn3AvUDmqfCEvf9YLNja4kWeA` |

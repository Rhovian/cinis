# cinis

Duel wager protocol on Solana. Two parties stake equal amounts of SPL tokens, a trusted authority declares the winner, and the loser's stake goes to the winner minus a platform fee.

Built with [Quasar](https://github.com/blueshift-gg/quasar) — zero-copy, zero-allocation.

## Instructions

### `create`
Challenger creates a duel, depositing their stake into a PDA-owned vault.

**Args:** `stake: u64`, `fee_bps: u16`, `expiry: i64`

### `accept`
Opponent accepts the duel by depositing a matching stake.

### `resolve`
Trusted authority declares a winner (`0` = challenger, `1` = opponent). Platform fee is deducted from the total pot, remainder goes to the winner. Closes the duel and vault.

**Args:** `winner: u8`

### `cancel`
- **Pending:** Challenger can cancel anytime. Stake returned.
- **Active:** Either party (challenger or opponent) can cancel. Each gets their stake back.

## State

**Duel** PDA — `seeds = [b"duel", challenger]`

| Field        | Type      | Description                          |
|-------------|-----------|--------------------------------------|
| challenger  | Address   | Duel creator                         |
| opponent    | Address   | Accepting party (zeroed until accept)|
| mint        | Address   | SPL token mint                       |
| authority   | Address   | Trusted third party for resolution   |
| fee_account | Address   | Platform fee token account           |
| stake       | u64       | Amount each party deposits           |
| expiry      | i64       | Optional expiry timestamp (0 = none) |
| fee_bps     | u16       | Platform fee in basis points         |
| status      | u8        | 0 = pending, 1 = active             |
| bump        | u8        | PDA bump seed                        |

## Limitations

- One active duel per challenger ([quasar#115](https://github.com/blueshift-gg/quasar/issues/115))

## Development

```sh
# check compilation (native)
cargo check -p cinis

# compile tests
cargo test --no-run -p cinis

# run tests (requires SBF build)
cargo build-sbf
cargo test -p cinis
```

## License

MIT

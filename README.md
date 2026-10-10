# Qubyte program

On-chain program behind [qubyte.app](https://qubyte.app). It launches pump.fun coins whose creator is authorised with a Falcon-512 signature, verified inside the program, and it turns every coin's creator fees into buybacks that are burned.

Program ID: `QUX1RmY5cmjf1o3dCAM8vKQ8goBxNzY3us1P3JNm62M`

## How it works

**Identity.** A creator registers a Falcon-512 public key once with `init_identity`. The program stores it in an `Identity` account at `["identity", 1, sha256(pubkey)]`, together with a nonce. That address is the creator's identity from then on; it holds no SOL and controls nothing except launches.

**Launch.** A Falcon signature (666 bytes) and pump.fun's `create_v2` accounts don't fit in one transaction, so a launch takes two:

1. `authorize_launch` verifies the signature, checks the nonce and expiry, consumes the nonce and records a `PendingLaunch` for the mint.
2. `launch` checks that the name, symbol and URI hash to what was signed, takes the service fee and the opening buy from the payer, creates the coin through pump.fun and writes its `CoinState`.

The payer is just the wallet that pays. It has no say over what is launched or under whose identity.

**Fees.** pump.fun's creator for every coin is that coin's own `fee_vault`, a PDA at `["fee_vault", mint]` with no private key. Creator fees can only leave it through `claim_and_burn`.

**Claim and burn.** `claim_and_burn` needs no signature, so anyone can run it. Of the fees claimed, 20% goes to the treasury and 80% buys the coin on its current venue and burns everything bought. Before migration the venue is the bonding curve; after it, the canonical PumpSwap pool. The split is written into `CoinState` at launch and can't be changed afterwards. Each launch also locks 0.01 SOL in `fee_vault`, and the coin's first claim spends all of it on a buyback.

## Signed message

```
message       = sha256("qubyte/launch/v2" ‖ program_id ‖ identity ‖ mint ‖ metadata_hash ‖ valid_until_slot ‖ nonce)
metadata_hash = sha256(len ‖ name ‖ len ‖ symbol ‖ len ‖ uri)
```

Lengths are u32 and integers are u64, all little-endian. Signatures are PQClean compressed Falcon-512 (header `0x39`), zero-padded to 666 bytes. A signature works once (nonce), only for that mint and metadata, and only until `valid_until_slot`.

## Instructions

| Instruction | Signed by | What it does |
| --- | --- | --- |
| `init_identity` | payer | Registers a Falcon public key as an `Identity` |
| `authorize_launch` | payer + Falcon | Verifies the launch signature, records `PendingLaunch` |
| `launch` | payer + mint | Creates the coin on pump.fun, writes `CoinState` |
| `claim_and_burn` | anyone | Splits claimed fees 80/20, buys back and burns |

`launch` expects pump.fun `create_v2`'s 16 accounts as remaining accounts. `claim_and_burn` expects the 17 accounts of the buy on its venue (`buy_exact_quote_in_v3` on the curve, PumpSwap `buy_exact_quote_in_v2` on the pool). Every account that decides where SOL or tokens go is checked against `fee_vault`, the mint and the canonical pool.

## Accounts

| Account | Seeds | Holds |
| --- | --- | --- |
| `Identity` | `"identity", 1, sha256(pubkey)` | Prepared Falcon key, nonce |
| `PendingLaunch` | `"pending_launch", mint` | Signer identity and metadata hash, closed by `launch` |
| `CoinState` | `"coin", mint` | Fee split, burn and buyback totals, launch time |
| `fee_vault` | `"fee_vault", mint` | Creator fees (no data, no key) |

## Building

Toolchain: Rust 1.89 (pinned in `rust-toolchain.toml`), Solana CLI 4.3, Anchor 1.2.1.

```sh
cargo build-sbf                         # target/deploy/qubyte.so
anchor build                            # also writes the IDL to target/idl
```

Tests run against the built program with [Mollusk](https://github.com/anza-xyz/mollusk) and sign with real Falcon keys:

```sh
cargo build-sbf
SBF_OUT_DIR=$PWD/target/deploy cargo test --features test-sbf --test test_qubyte -- --nocapture
```

They cover identity registration and the launch's authorisation paths: wrong key, wrong mint, replay, expiry, and metadata swapped after signing. The pump.fun side (launch, collecting fees, buybacks on the curve and on PumpSwap, migration) was tested end to end on a mainnet fork.

## Verifying the deployment

The deployed program is built with [solana-verify](https://github.com/Ellipsis-Labs/solana-verifiable-build) from this repository, so anyone can check that the bytecode on chain matches this source:

```sh
solana-verify build --library-name qubyte
solana-verify get-executable-hash target/deploy/qubyte.so
solana-verify get-program-hash -u mainnet-beta QUX1RmY5cmjf1o3dCAM8vKQ8goBxNzY3us1P3JNm62M
```

The two hashes should be equal. To verify against the repository directly:

```sh
solana-verify verify-from-repo -u mainnet-beta \
  --program-id QUX1RmY5cmjf1o3dCAM8vKQ8goBxNzY3us1P3JNm62M \
  --library-name qubyte \
  https://github.com/qubyteapp/qubyte-program
```

`Cargo.lock` is committed and the build has no network access, so the result depends only on the commit you build.

## Compute and size

`authorize_launch` uses about 290k compute units, mostly Falcon verification. Clients need to raise the compute limit above the default 200k. The largest transaction, `init_identity`, is 1,180 bytes out of 1,232, because it carries the 897-byte public key. That's also why the program has no proof of possession: the key and a signature together don't fit in one transaction. It doesn't need one anyway, since the identity address is derived from the key itself.

## Limits

- pump.fun is upgradeable and outside this program's control. Its admin can reassign a coin's creator, which would redirect future fees.
- Buybacks have no slippage limit and spend everything claimed at once.
- The program has not been audited.

# Upgrade Guide

This document describes how to safely upgrade the StellarSave Soroban contract and how the automated upgrade test suite protects backward compatibility.

## Overview

Soroban contracts are upgraded by deploying a new WASM binary to the same contract address using `stellar contract install` + `stellar contract upgrade`. Because all on-chain state persists across upgrades, every new version must be able to read data written by every previous version.

## Upgrade Test Suite

The upgrade tests live in `contracts/stellar-save/src/upgrade_tests.rs` and are registered as the `upgrade_tests` module in `lib.rs`. They run automatically on every push or pull request that touches `contracts/**` via the `.github/workflows/upgrade-tests.yml` workflow.

### Test categories

| Category | What it checks |
|---|---|
| Storage backward compatibility | Data seeded in the v0 schema is deserialised correctly by the current code |
| API stability | All public entry-points (`get_member_count`, `update_config`, `validate_contribution_amount`) accept v0 data without panicking |
| Migration defaults | New fields added to existing structs carry safe zero/false defaults on old records |
| Performance regression | Key read paths remain O(1) regardless of how many members or contributions exist |

### Running locally

```bash
# Run only the upgrade tests
cargo test --manifest-path contracts/stellar-save/Cargo.toml upgrade_tests -- --test-threads=1

# Run with coverage
cargo llvm-cov --manifest-path contracts/stellar-save/Cargo.toml \
  -- upgrade_tests --test-threads=1
```

## Upgrade Procedure

### 1. Pre-upgrade checklist

- [ ] All upgrade tests pass on the new branch (`cargo test upgrade_tests`)
- [ ] Full test suite passes (`cargo test -- --test-threads=1`)
- [ ] WASM binary builds cleanly for `wasm32-unknown-unknown`
- [ ] Storage schema changes are documented in the section below
- [ ] Any new struct fields have `#[contracttype]` and a sensible default

### 2. Build the new WASM

```bash
cargo build \
  --manifest-path contracts/stellar-save/Cargo.toml \
  --target wasm32-unknown-unknown \
  --release
```

The optimised binary is at:
```
target/wasm32-unknown-unknown/release/stellar_save.wasm
```

### 3. Install and upgrade on testnet

```bash
# Install the new WASM and capture the new hash
NEW_HASH=$(stellar contract install \
  --network testnet \
  --source deployer \
  --wasm target/wasm32-unknown-unknown/release/stellar_save.wasm)

echo "New WASM hash: $NEW_HASH"

# Upgrade the live contract to the new WASM
stellar contract invoke \
  --network testnet \
  --source deployer \
  --id "$CONTRACT_ID" \
  -- upgrade \
  --new_wasm_hash "$NEW_HASH"
```

### 4. Verify after upgrade

```bash
# Smoke-test: read an existing group to confirm storage is intact
stellar contract invoke \
  --network testnet \
  --id "$CONTRACT_ID" \
  -- get_member_count \
  --group_id 1
```

### 5. Mainnet upgrade

Repeat steps 2–4 against `--network mainnet`. Mainnet upgrades require the `production` GitHub environment approval gate defined in `ci.yml`.

## Storage Schema

### v0.1.0 (current)

| Key builder | Type | Notes |
|---|---|---|
| `group_data(id)` | `Group` | Core group config and state |
| `group_status(id)` | `GroupStatus` | Lifecycle state enum |
| `group_members(id)` | `Vec<Address>` | Ordered member list |
| `member_profile(id, addr)` | `MemberProfile` | Per-member metadata |
| `contribution_individual(id, cycle, addr)` | `ContributionRecord` | Single contribution |
| `contribution_cycle_total(id, cycle)` | `i128` | Pre-computed cycle sum |
| `contribution_cycle_count(id, cycle)` | `u32` | Pre-computed contributor count |
| `group_balance(id)` | `i128` | Running group balance |
| `next_group_id()` | `u64` | Monotonic ID counter |
| `contract_config()` | `ContractConfig` | Global limits and admin |

### Adding a new field to an existing struct

1. Add the field with a default value using `Option<T>` or a primitive that defaults to `0`/`false`.
2. Add a test in `upgrade_tests.rs` that seeds the old record (without the new field) and asserts the default is correct after deserialization.
3. Update the schema table above.

### Adding a new storage key

1. Add the key variant to `StorageKey` in `storage.rs`.
2. Add a builder method to `StorageKeyBuilder`.
3. No migration is needed — the key simply won't exist on old deployments and callers must handle `None`.

## Rollback

Soroban does not support automatic rollback. If a bad upgrade is deployed:

1. Build the previous WASM from the last known-good git tag.
2. Install it and upgrade back using the same procedure above.
3. If storage was mutated in an incompatible way, a manual data-repair transaction may be required — contact the contract admin.

## Version History

| Version | Date | Changes |
|---|---|---|
| 0.1.0 | 2026-04-25 | Initial release — XLM-only ROSCA groups |

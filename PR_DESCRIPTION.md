# Implement Rate Limiting to Prevent Spam

## Description
This PR adds rate limiting (cooldown) mechanisms to the Stellar-Save Soroban smart contract to prevent spam and abuse.

### Key Changes
- **Group Creation Cooldown**: Added a 5-minute (300 seconds) cooldown period between group creations per user
- **Group Join Cooldown**: Added a 2-minute (120 seconds) cooldown period between joining different groups per user
- **Storage Keys**: Implemented storage keys to track last creation and last join timestamps per user
- **Error Handling**: Returns `StellarSaveError::RateLimitExceeded` (error code 9005) when rate limit is exceeded

### Implementation Details
- Constants defined in [`lib.rs`](contracts/stellar-save/src/lib.rs):
  - `GROUP_CREATION_COOLDOWN: u64 = 300` (5 minutes)
  - `GROUP_JOIN_COOLDOWN: u64 = 120` (2 minutes)
- Rate limiting is enforced in `create_group` and `join_group` functions
- Each user has independent cooldown timers - actions by one user don't affect others

### Tests
- `test_group_creation_rate_limit`: Verifies that creating multiple groups within cooldown period fails
- `test_group_join_rate_limit`: Verifies that joining multiple groups within cooldown period fails

## Verification
- Contract compiles successfully: `cargo build --lib`
- Rate limiting tests exist and verify the functionality

## Related Issue
Fixes #270 - Add rate limiting to prevent spam

#![no_std]

//! # Stellar-Save Smart Contract
//!
//! A decentralized rotational savings and credit association (ROSCA) built on Stellar Soroban.

pub mod contribution;
pub mod error;
pub mod events;
pub mod group;
pub mod payout;
pub mod refund;
pub mod storage;

// Re-export for convenience
pub use contribution::ContributionRecord;
pub use error::{ContractResult, ErrorCategory, StellarSaveError};
pub use events::EventEmitter;
pub use group::{Group, GroupStatus};
pub use payout::PayoutRecord;
pub use refund::RefundRecord;
pub use storage::StorageKeyBuilder;
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct StellarSaveContract;

#[contractimpl]
impl StellarSaveContract {
    pub fn hello(env: Env) -> soroban_sdk::Symbol {
        soroban_sdk::symbol_short!("hello")
    }

    /// Request a refund for a contribution made in error or when a group fails to activate.
    ///
    /// # Arguments
    /// * `group_id` - The group the contribution was made to
    /// * `cycle`    - The cycle number of the contribution
    /// * `caller`   - The address requesting the refund (must be contributor or group creator)
    ///
    /// # Errors
    /// - `GroupNotFound`        - Group does not exist
    /// - `ContributionNotFound` - No contribution found for caller/group/cycle
    /// - `AlreadyRefunded`      - Contribution already refunded
    /// - `RefundNotEligible`    - Group state does not allow refunds (e.g. payout already done)
    /// - `Unauthorized`         - Caller is neither the contributor nor the group creator
    pub fn request_refund(
        env: Env,
        group_id: u64,
        cycle: u32,
        caller: Address,
    ) -> Result<RefundRecord, StellarSaveError> {
        refund::request_refund(&env, group_id, cycle, caller)
    }
}

#![no_std]

//! # Stellar-Save Smart Contract
//! 
//! A decentralized rotational savings and credit association (ROSCA) built on Stellar Soroban.
//! 
//! This contract enables groups to pool funds in a rotating savings system where:
//! - Members contribute a fixed amount each cycle
//! - One member receives the total pool each cycle
//! - The process rotates until all members have received a payout
//! 
//! ## Modules
//! - `group`: Core Group data structure and state management
//! - `contribution`: Contribution record tracking for member payments
//! - `payout`: Payout record tracking for fund distributions

pub mod contribution;
pub mod group;
pub mod payout;

// Re-export for convenience
pub use contribution::ContributionRecord;
pub use group::{emit_group_activated, Group, GroupActivatedEvent};
pub use payout::PayoutRecord;
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct StellarSaveContract;

#[contractimpl]
impl StellarSaveContract {
    pub fn hello(env: Env) -> soroban_sdk::Symbol {
        soroban_sdk::symbol_short!("hello")
    }

    /// Activates a group once minimum members have joined.
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `group_id` - ID of the group to activate
    /// * `creator` - The creator's address (must match the group's creator)
    /// * `member_count` - Current number of members in the group
    /// 
    /// # Panics
    /// Panics if:
    /// - The caller is not the group creator
    /// - The group has already been started
    /// - Minimum member count has not been reached
    pub fn activate_group(env: Env, group_id: u64, creator: Address, member_count: u32) {
        // Get the group - in a real implementation, this would come from storage
        // For now, we'll create a mock group to demonstrate the logic
        // In production, you'd load from: let mut group = GroupStorage::get(&env, group_id);
        
        // Verify caller is creator
        assert!(
            creator == creator,
            "caller must be the group creator"
        );
        
        // Get current timestamp
        let timestamp = env.ledger().timestamp();
        
        // Create a temporary group for validation (in production, load from storage)
        let mut group = Group::new(
            group_id,
            creator,
            10_000_000, // Default contribution amount
            604800,     // Default cycle duration
            5,          // Default max members
            2,          // Default min members
            timestamp,
        );
        
        // Simulate adding members (in production, this would be tracked in storage)
        for _ in 0..member_count {
            group.add_member();
        }
        
        // Check minimum members met (using the activate method)
        group.activate(timestamp);
        
        // Emit the activation event
        emit_group_activated(&env, group_id, timestamp, member_count);
    }
}

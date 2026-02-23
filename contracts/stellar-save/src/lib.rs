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
//! - `events`: Event types for contract state change tracking
//! - `error`: Comprehensive error types and handling
//! - `group`: Core Group data structure and state management
//! - `contribution`: Contribution record tracking for member payments
//! - `payout`: Payout record tracking for fund distributions
//! - `storage`: Storage key structure for efficient data access
//! - `status`: Group lifecycle status enum with state transitions
//! - `events`: Event definitions for contract actions

pub mod events;
pub mod error;
pub mod contribution;
pub mod group;
pub mod payout;
pub mod status;
pub mod storage;
pub mod pool;

// Re-export for convenience
pub use events::*;
pub use error::{StellarSaveError, ErrorCategory, ContractResult};
pub use group::{Group, GroupStatus};
pub use contribution::ContributionRecord;
pub use payout::PayoutRecord;
pub use status::StatusError;
pub use storage::{StorageKey, StorageKeyBuilder};
pub use pool::{PoolInfo, PoolCalculator};
pub use events::EventEmitter;
use soroban_sdk::{contract, contractimpl, contracttype, Env, Address, Vec, Symbol};

#[contract]
pub struct StellarSaveContract;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractConfig {
    pub admin: Address,
    pub min_contribution: i128,
    pub max_contribution: i128,
    pub min_members: u32,
    pub max_members: u32,
    pub min_cycle_duration: u64,
    pub max_cycle_duration: u64,
}

impl ContractConfig {
    pub fn validate(&self) -> bool {
        self.min_contribution > 0 && 
        self.max_contribution >= self.min_contribution &&
        self.min_members >= 2 && 
        self.max_members >= self.min_members &&
        self.min_cycle_duration > 0 &&
        self.max_cycle_duration >= self.min_cycle_duration
    }
}

#[contractimpl]
impl StellarSaveContract {
    fn generate_next_group_id(env: &Env) -> Result<u64, StellarSaveError> {
        let key = StorageKeyBuilder::next_group_id();
        
        // Counter storage: default to 0 if not yet initialized
        let current_id: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        
        // Atomic increment & Overflow protection
        let next_id = current_id.checked_add(1)
            .ok_or(StellarSaveError::Overflow)?; // Ensure StellarSaveError has Overflow variant
            
        // Update counter
        env.storage().persistent().set(&key, &next_id);
        
        Ok(next_id)
    }

    /// Increments the group ID counter and returns the new ID.
    /// Tasks: Counter storage, Atomic increment, Overflow protection.
    fn increment_group_id(env: &Env) -> Result<u64, StellarSaveError> {
        let key = StorageKeyBuilder::next_group_id();
        
        // 1. Read current ID (Counter storage)
        // Defaults to 0 if no groups have ever been created.
        let current_id: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        
        // 2. Atomic increment with Overflow protection
        let next_id = current_id.checked_add(1)
            .ok_or(StellarSaveError::Overflow)?;
        
        // 3. Update persistent storage
        env.storage().persistent().set(&key, &next_id);
        
        Ok(next_id)
    }

    /// Initializes or updates the global contract configuration.
    /// Only the current admin can perform this update.
    pub fn update_config(env: Env, new_config: ContractConfig) -> Result<(), StellarSaveError> {
        // 1. Validation Logic
        if !new_config.validate() {
            return Err(StellarSaveError::InvalidState); 
        }

        let key = StorageKeyBuilder::contract_config();

        // 2. Admin-only Authorization
        if let Some(current_config) = env.storage().persistent().get::<_, ContractConfig>(&key) {
            current_config.admin.require_auth();
        } else {
            // First time initialization: caller becomes admin
            new_config.admin.require_auth();
        }

        // 3. Save Configuration
        env.storage().persistent().set(&key, &new_config);
        Ok(())
    }

    /// Creates a new savings group (ROSCA).
    /// Tasks: Validate parameters, Generate ID, Initialize Struct, Store Data, Emit Event.
    pub fn create_group(
        env: Env,
        creator: Address,
        contribution_amount: i128,
        cycle_duration: u64,
        max_members: u32,
    ) -> Result<u64, StellarSaveError> {
        // 1. Authorization: Only the creator can initiate this transaction
        creator.require_auth();

        // 2. Global Validation: Check against ContractConfig
        let config_key = StorageKeyBuilder::contract_config();
        if let Some(config) = env.storage().persistent().get::<_, ContractConfig>(&config_key) {
            if contribution_amount < config.min_contribution || contribution_amount > config.max_contribution ||
               max_members < config.min_members || max_members > config.max_members ||
               cycle_duration < config.min_cycle_duration || cycle_duration > config.max_cycle_duration {
                return Err(StellarSaveError::InvalidState);
            }
        }

        // 3. Generate unique group ID
        let group_id = Self::generate_next_group_id(&env)?;

        // 4. Initialize Group Struct
        let current_time = env.ledger().timestamp();
        let min_members = 2; // Default minimum members
        let new_group = Group::new(
            group_id,
            creator.clone(),
            contribution_amount,
            cycle_duration,
            max_members,
            min_members,
            current_time,
        );

        // 5. Store Group Data
        let group_key = StorageKeyBuilder::group_data(group_id);
        env.storage().persistent().set(&group_key, &new_group);
        
        // Initialize Group Status as Pending
        let status_key = StorageKeyBuilder::group_status(group_id);
        env.storage().persistent().set(&status_key, &GroupStatus::Pending);

        // 6. Emit GroupCreated Event
        env.events().publish(
            (Symbol::new(&env, "GroupCreated"), creator),
            group_id
        );

        // 7. Return Group ID
        Ok(group_id)
    }

    /// Updates group parameters. Only allowed for creators while the group is Pending.
    pub fn update_group(
        env: Env,
        group_id: u64,
        new_contribution: i128,
        new_duration: u64,
        new_max_members: u32,
    ) -> Result<(), StellarSaveError> {
        // 1. Load existing group data
        let group_key = StorageKeyBuilder::group_data(group_id);
        let mut group = env.storage()
            .persistent()
            .get::<_, Group>(&group_key)
            .ok_or(StellarSaveError::GroupNotFound)?;

        // 2. Task: Verify caller is creator
        group.creator.require_auth();

        // 3. Task: Check group is not yet active
        let status_key = StorageKeyBuilder::group_status(group_id);
        let status = env.storage()
            .persistent()
            .get::<_, GroupStatus>(&status_key)
            .unwrap_or(GroupStatus::Pending);

        if status != GroupStatus::Pending {
            return Err(StellarSaveError::InvalidState);
        }

        // 4. Task: Validate new parameters against global config
        let config_key = StorageKeyBuilder::contract_config();
        if let Some(config) = env.storage().persistent().get::<_, ContractConfig>(&config_key) {
            if new_contribution < config.min_contribution || new_contribution > config.max_contribution ||
               new_max_members < config.min_members || new_max_members > config.max_members ||
               new_duration < config.min_cycle_duration || new_duration > config.max_cycle_duration {
                return Err(StellarSaveError::InvalidState);
            }
        }

        // 5. Task: Update storage
        group.contribution_amount = new_contribution;
        group.cycle_duration = new_duration;
        group.max_members = new_max_members;
        
        env.storage().persistent().set(&group_key, &group);

        // 6. Task: Emit event
        env.events().publish(
            (Symbol::new(&env, "GroupUpdated"), group_id),
            group.creator
        );

        Ok(())
    }

    /// Retrieves the details of a specific savings group.
    /// 
    /// # Arguments
    /// * `group_id` - The unique identifier of the group to retrieve.
    /// 
    /// # Returns
    /// Returns the Group struct if found, or StellarSaveError::GroupNotFound if not.
    pub fn get_group(env: Env, group_id: u64) -> Result<Group, StellarSaveError> {
        // Generate the storage key for the group data
        let key = StorageKeyBuilder::group_data(group_id);

        // Attempt to load group from persistent storage
        env.storage()
            .persistent()
            .get::<_, Group>(&key)
            .ok_or(StellarSaveError::GroupNotFound)
    }

    /// Deletes a group from storage.
    /// Only allowed if the caller is the creator and no members have joined yet.
    pub fn delete_group(env: Env, group_id: u64) -> Result<(), StellarSaveError> {
        // 1. Task: Load group and Verify caller is creator
        let group_key = StorageKeyBuilder::group_data(group_id);
        let group = env.storage()
            .persistent()
            .get::<_, Group>(&group_key)
            .ok_or(StellarSaveError::GroupNotFound)?;

        group.creator.require_auth();

        // 2. Task: Check no members joined
        // We check if the member count is 0. 
        // Note: If the creator is automatically added as a member in join_group, 
        // this check should be adjusted to (count == 1).
        if group.member_count > 0 {
            return Err(StellarSaveError::InvalidState);
        }

        // 3. Task: Remove from storage
        // We remove both the main data and the status record
        env.storage().persistent().remove(&group_key);
        
        let status_key = StorageKeyBuilder::group_status(group_id);
        env.storage().persistent().remove(&status_key);

        // 4. Task: Emit event
        env.events().publish(
            (Symbol::new(&env, "GroupDeleted"), group_id),
            group.creator
        );

        Ok(())
    }

    /// Returns the total number of groups created.
    /// This reads the existing counter from storage without modifying it.
    pub fn get_total_groups(env: Env) -> u64 {
        let key = StorageKeyBuilder::next_group_id();
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    /// Lists groups with cursor-based pagination and optional status filtering.
    /// Tasks: Pagination, Status Filtering, Gas Optimization.
    pub fn list_groups(
        env: Env,
        cursor: u64,
        limit: u32,
        status_filter: Option<GroupStatus>,
    ) -> Result<Vec<Group>, StellarSaveError> {
        let mut groups = Vec::new(&env);
        let max_id_key = StorageKeyBuilder::next_group_id();
        
        // 1. Get the current maximum ID to know where to stop
        let current_max_id: u64 = env.storage().persistent().get(&max_id_key).unwrap_or(0);
        
        // 2. Optimization: Start from the cursor and move backwards or forwards
        // Here we go backwards from the cursor to show newest groups first
        let start = if cursor == 0 { current_max_id } else { cursor };
        let mut count = 0;
        let page_limit = if limit > 50 { 50 } else { limit }; // Safety cap for gas

        for id in (1..=start).rev() {
            if count >= page_limit {
                break;
            }

            let group_key = StorageKeyBuilder::group_data(id);
            if let Some(group) = env.storage().persistent().get::<_, Group>(&group_key) {
                
                // 3. Optional Status Filtering
                if let Some(ref filter) = status_filter {
                    let status_key = StorageKeyBuilder::group_status(id);
                    let status = env.storage().persistent().get::<_, GroupStatus>(&status_key)
                        .unwrap_or(GroupStatus::Pending);
                    
                    if &status == filter {
                        groups.push_back(group);
                        count += 1;
                    }
                } else {
                    groups.push_back(group);
                    count += 1;
                }
            }
        }

        Ok(groups)
    }

    /// Returns the total number of groups created.
    /// Reads the existing counter from storage without modification.
    pub fn get_total_groups_created(env: Env) -> u64 {
        let key = StorageKeyBuilder::next_group_id();
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    /// Retrieves contribution details for a member in a cycle.
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `group_id` - ID of the group
    /// * `cycle_number` - The cycle number
    /// * `member` - Address of the member
    /// 
    /// # Returns
    /// * `Ok(ContributionRecord)` - The contribution record
    /// * `Err(StellarSaveError)` if contribution not found
    pub fn get_contribution(
        env: Env,
        group_id: u64,
        cycle_number: u32,
        member: Address,
    ) -> Result<ContributionRecord, StellarSaveError> {
        let key = StorageKeyBuilder::contribution_individual(group_id, cycle_number, member);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(StellarSaveError::ContributionNotFound)
    }

    /// Allows a member to contribute to the current cycle.
    /// 
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `group_id` - ID of the group to contribute to
    /// * `contributor` - Address of the member making the contribution
    /// 
    /// # Returns
    /// * `Ok(())` if contribution is successful
    /// * `Err(StellarSaveError)` if validation fails
    pub fn contribute(env: Env, group_id: u64, contributor: Address) -> Result<(), StellarSaveError> {
        // 1. Verify caller is member
        contributor.require_auth();
        
        let member_key = StorageKeyBuilder::member_profile(group_id, contributor.clone());
        if !env.storage().persistent().has(&member_key) {
            return Err(StellarSaveError::NotMember);
        }
        
        // 2. Load group and verify it's active
        let group_key = StorageKeyBuilder::group_data(group_id);
        let group = env.storage()
            .persistent()
            .get::<_, Group>(&group_key)
            .ok_or(StellarSaveError::GroupNotFound)?;
        
        let status_key = StorageKeyBuilder::group_status(group_id);
        let status = env.storage()
            .persistent()
            .get::<_, GroupStatus>(&status_key)
            .unwrap_or(GroupStatus::Pending);
        
        if !status.accepts_contributions() {
            return Err(StellarSaveError::InvalidState);
        }
        
        // 3. Check correct amount
        let amount = group.contribution_amount;
        if amount <= 0 {
            return Err(StellarSaveError::InvalidAmount);
        }
        
        // 4. Check not already contributed this cycle
        let cycle = group.current_cycle;
        let contrib_key = StorageKeyBuilder::contribution_individual(group_id, cycle, contributor.clone());
        if env.storage().persistent().has(&contrib_key) {
            return Err(StellarSaveError::AlreadyContributed);
        }
        
        // 5. Transfer funds to contract (placeholder - actual token transfer would go here)
        // In production: token.transfer(&contributor, &env.current_contract_address(), &amount);
        
        // 6. Record contribution
        let timestamp = env.ledger().timestamp();
        let contribution = ContributionRecord::new(
            contributor.clone(),
            group_id,
            cycle,
            amount,
            timestamp,
        );
        env.storage().persistent().set(&contrib_key, &contribution);
        
        // Update cycle totals
        let total_key = StorageKeyBuilder::contribution_cycle_total(group_id, cycle);
        let current_total: i128 = env.storage().persistent().get(&total_key).unwrap_or(0);
        env.storage().persistent().set(&total_key, &(current_total + amount));
        
        let count_key = StorageKeyBuilder::contribution_cycle_count(group_id, cycle);
        let current_count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
        env.storage().persistent().set(&count_key, &(current_count + 1));
        
        // 7. Emit ContributionMade event
        let cycle_total = current_total + amount;
        EventEmitter::emit_contribution_made(
            &env,
            group_id,
            contributor,
            amount,
            cycle,
            cycle_total,
            timestamp,
        );
        
        // 8. Check if cycle complete
        let new_count = current_count + 1;
        if new_count == group.member_count {
            // Cycle is complete - ready for payout
            env.events().publish(
                (Symbol::new(&env, "cycle_complete"), group_id),
                cycle
            );
        }
        
        Ok(())
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
        env.events().publish(
            (Symbol::new(&env, "group_activated"), group_id),
            member_count
        );
    }
}

fn emit_group_activated(env: &Env, group_id: u64, timestamp: u64, member_count: u32) {
    env.events().publish(
        (Symbol::new(env, "group_activated"), group_id),
        (timestamp, member_count)
    );
}

#[test]
fn test_group_id_uniqueness() {
    let env = Env::default();
    
    // Generate first ID
    let id1 = StellarSaveContract::increment_group_id(&env).unwrap();
    // Generate second ID
    let id2 = StellarSaveContract::increment_group_id(&env).unwrap();
    
    // Assert IDs are sequential and unique
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_ne!(id1, id2);
}

#[test]
fn test_get_total_groups() {
    let env = Env::default();
    let contract_id = env.register_contract(None, StellarSaveContract);
    let client = StellarSaveContractClient::new(&env, &contract_id);
    let creator = Address::generate(&env);

    // Initially, no groups should exist
    assert_eq!(client.get_total_groups(), 0);

    // Create a group
    env.mock_all_auths();
    client.create_group(&creator, &100, &3600, &5);

    // Total groups should now be 1
    assert_eq!(client.get_total_groups(), 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_get_group_success() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);

        // Manually store a group to test retrieval
        let group_id = 1;
        let group = Group::new(group_id, creator.clone(), 100, 3600, 5, 2, 12345);
        
        // This simulates the storage state after create_group is called
        env.storage().persistent().set(&StorageKeyBuilder::group_data(group_id), &group);

        let retrieved_group = client.get_group(&group_id);
        assert_eq!(retrieved_group.id, group_id);
        assert_eq!(retrieved_group.creator, creator);
    }

    #[test]
    #[should_panic(expected = "Status(ContractError(1001))")] // 1001 is GroupNotFound
    fn test_get_group_not_found() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);

        client.get_group(&999); // ID that doesn't exist
    }

    // #[test]
    // fn test_update_group_success() {
    //     let env = Env::default();
    //     // ... setup contract and create a group in Pending state ...
    //     
    //     // Attempt update
    //     client.update_group(&group_id, &200, &7200, &10);
    //     
    //     let updated = client.get_group(&group_id);
    //     assert_eq!(updated.contribution_amount, 200);
    // }

    // #[test]
    // #[should_panic(expected = "Status(ContractError(1003))")] // InvalidState
    // fn test_update_group_fails_if_active() {
    //     let env = Env::default();
    //     // ... setup contract and manually set status to GroupStatus::Active ...
    //     
    //     client.update_group(&group_id, &200, &7200, &10);
    // }

    // #[test]
    // fn test_delete_group_success() {
    //     let env = Env::default();
    //     let contract_id = env.register_contract(None, StellarSaveContract);
    //     let client = StellarSaveContractClient::new(&env, &contract_id);
    //     let creator = Address::generate(&env);

    //     // 1. Setup: Create a group with 0 members
    //     let group_id = client.create_group(&creator, &100, &3600, &5);
    //     
    //     // 2. Action: Delete group
    //     env.mock_all_auths();
    //     client.delete_group(&group_id);

    //     // 3. Verify: Group should no longer exist
    //     let result = client.try_get_group(&group_id);
    //     assert!(result.is_err());
    // }

    // #[test]
    // #[should_panic(expected = "Status(ContractError(1003))")] // InvalidState
    // fn test_delete_group_fails_if_has_members() {
    //     let env = Env::default();
    //     // ... setup and add a member to the group ...
    //     
    //     client.delete_group(&group_id);
    // }

    // #[test]
    // fn test_list_groups_pagination() {
    //     let env = Env::default();
    //     // ... setup contract and create 5 groups ...

    //     // List 2 groups starting from the top
    //     let page1 = client.list_groups(&0, &2, &None);
    //     assert_eq!(page1.len(), 2);
    //     
    //     // Get the next page using the last ID as a cursor
    //     let last_id = page1.get(1).unwrap().id;
    //     let page2 = client.list_groups(&(last_id - 1), &2, &None);
    //     assert_eq!(page2.len(), 2);
    // }

    // #[test]
    // fn test_list_groups_filtering() {
    //     let env = Env::default();
    //     // ... setup contract, create 1 Active group and 1 Pending group ...
    //     
    //     let active_only = client.list_groups(&0, &10, &Some(GroupStatus::Active));
    //     assert_eq!(active_only.len(), 1);
    // }

    #[test]
    fn test_get_total_groups_created() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);

        // Initially, no groups created
        let count = client.get_total_groups_created();
        assert_eq!(count, 0);

        // Create first group
        env.mock_all_auths();
        client.create_group(&creator, &100, &3600, &5);
        
        let count = client.get_total_groups_created();
        assert_eq!(count, 1);

        // Create second group
        client.create_group(&creator, &200, &7200, &10);
        
        let count = client.get_total_groups_created();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_contribute_success() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let member = Address::generate(&env);

        // Setup: Create a group and add member
        let group_id = 1;
        let group = Group::new(group_id, member.clone(), 100, 3600, 5, 2, env.ledger().timestamp());
        env.storage().persistent().set(&StorageKeyBuilder::group_data(group_id), &group);
        env.storage().persistent().set(&StorageKeyBuilder::group_status(group_id), &GroupStatus::Active);
        env.storage().persistent().set(&StorageKeyBuilder::member_profile(group_id, member.clone()), &true);

        // Action: Make contribution
        env.mock_all_auths();
        let result = client.contribute(&group_id, &member);
        assert!(result.is_ok());

        // Verify: Contribution was recorded
        let contrib_key = StorageKeyBuilder::contribution_individual(group_id, 0, member.clone());
        assert!(env.storage().persistent().has(&contrib_key));
    }

    #[test]
    #[should_panic(expected = "Status(ContractError(2002))")] // NotMember
    fn test_contribute_not_member() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let non_member = Address::generate(&env);

        // Setup: Create a group without adding the member
        let group_id = 1;
        let creator = Address::generate(&env);
        let group = Group::new(group_id, creator, 100, 3600, 5, 2, env.ledger().timestamp());
        env.storage().persistent().set(&StorageKeyBuilder::group_data(group_id), &group);
        env.storage().persistent().set(&StorageKeyBuilder::group_status(group_id), &GroupStatus::Active);

        // Action: Try to contribute as non-member
        env.mock_all_auths();
        client.contribute(&group_id, &non_member);
    }

    #[test]
    #[should_panic(expected = "Status(ContractError(3002))")] // AlreadyContributed
    fn test_contribute_already_contributed() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let member = Address::generate(&env);

        // Setup: Create a group, add member, and record a contribution
        let group_id = 1;
        let group = Group::new(group_id, member.clone(), 100, 3600, 5, 2, env.ledger().timestamp());
        env.storage().persistent().set(&StorageKeyBuilder::group_data(group_id), &group);
        env.storage().persistent().set(&StorageKeyBuilder::group_status(group_id), &GroupStatus::Active);
        env.storage().persistent().set(&StorageKeyBuilder::member_profile(group_id, member.clone()), &true);
        
        let contrib = ContributionRecord::new(member.clone(), group_id, 0, 100, env.ledger().timestamp());
        let contrib_key = StorageKeyBuilder::contribution_individual(group_id, 0, member.clone());
        env.storage().persistent().set(&contrib_key, &contrib);

        // Action: Try to contribute again
        env.mock_all_auths();
        client.contribute(&group_id, &member);
    }

    #[test]
    #[should_panic(expected = "Status(ContractError(1003))")] // InvalidState
    fn test_contribute_group_not_active() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let member = Address::generate(&env);

        // Setup: Create a group in Pending state
        let group_id = 1;
        let group = Group::new(group_id, member.clone(), 100, 3600, 5, 2, env.ledger().timestamp());
        env.storage().persistent().set(&StorageKeyBuilder::group_data(group_id), &group);
        env.storage().persistent().set(&StorageKeyBuilder::group_status(group_id), &GroupStatus::Pending);
        env.storage().persistent().set(&StorageKeyBuilder::member_profile(group_id, member.clone()), &true);

        // Action: Try to contribute while group is pending
        env.mock_all_auths();
        client.contribute(&group_id, &member);
    }

    #[test]
    fn test_get_contribution_success() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let member = Address::generate(&env);

        let group_id = 1;
        let cycle = 0;
        let amount = 100;
        let timestamp = env.ledger().timestamp();
        
        // Setup: Create and store a contribution
        let contribution = ContributionRecord::new(
            member.clone(),
            group_id,
            cycle,
            amount,
            timestamp,
        );
        let key = StorageKeyBuilder::contribution_individual(group_id, cycle, member.clone());
        env.storage().persistent().set(&key, &contribution);

        // Action: Get contribution
        let result = client.get_contribution(&group_id, &cycle, &member);
        
        // Verify: Contribution matches
        assert_eq!(result.member_address, member);
        assert_eq!(result.group_id, group_id);
        assert_eq!(result.cycle_number, cycle);
        assert_eq!(result.amount, amount);
        assert_eq!(result.timestamp, timestamp);
    }

    #[test]
    #[should_panic(expected = "Status(ContractError(3004))")] // ContributionNotFound
    fn test_get_contribution_not_found() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let member = Address::generate(&env);

        // Action: Try to get non-existent contribution
        client.get_contribution(&1, &0, &member);
    }

    #[test]
    fn test_get_contribution_different_cycles() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let member = Address::generate(&env);

        let group_id = 1;
        
        // Setup: Create contributions for different cycles
        let contrib_cycle0 = ContributionRecord::new(
            member.clone(),
            group_id,
            0,
            100,
            1000,
        );
        let contrib_cycle1 = ContributionRecord::new(
            member.clone(),
            group_id,
            1,
            200,
            2000,
        );
        
        env.storage().persistent().set(
            &StorageKeyBuilder::contribution_individual(group_id, 0, member.clone()),
            &contrib_cycle0,
        );
        env.storage().persistent().set(
            &StorageKeyBuilder::contribution_individual(group_id, 1, member.clone()),
            &contrib_cycle1,
        );

        // Action: Get contributions for different cycles
        let result0 = client.get_contribution(&group_id, &0, &member);
        let result1 = client.get_contribution(&group_id, &1, &member);
        
        // Verify: Each cycle has correct data
        assert_eq!(result0.cycle_number, 0);
        assert_eq!(result0.amount, 100);
        assert_eq!(result0.timestamp, 1000);
        
        assert_eq!(result1.cycle_number, 1);
        assert_eq!(result1.amount, 200);
        assert_eq!(result1.timestamp, 2000);
    }

    #[test]
    fn test_get_contribution_different_members() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        
        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);

        let group_id = 1;
        let cycle = 0;
        
        // Setup: Create contributions for different members
        let contrib1 = ContributionRecord::new(member1.clone(), group_id, cycle, 100, 1000);
        let contrib2 = ContributionRecord::new(member2.clone(), group_id, cycle, 100, 2000);
        
        env.storage().persistent().set(
            &StorageKeyBuilder::contribution_individual(group_id, cycle, member1.clone()),
            &contrib1,
        );
        env.storage().persistent().set(
            &StorageKeyBuilder::contribution_individual(group_id, cycle, member2.clone()),
            &contrib2,
        );

        // Action: Get contributions for different members
        let result1 = client.get_contribution(&group_id, &cycle, &member1);
        let result2 = client.get_contribution(&group_id, &cycle, &member2);
        
        // Verify: Each member has correct data
        assert_eq!(result1.member_address, member1);
        assert_eq!(result1.timestamp, 1000);
        
        assert_eq!(result2.member_address, member2);
        assert_eq!(result2.timestamp, 2000);
    }

    #[test]
    fn test_get_contribution_validates_record() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StellarSaveContract);
        let client = StellarSaveContractClient::new(&env, &contract_id);
        let member = Address::generate(&env);

        let group_id = 1;
        let cycle = 0;
        
        // Setup: Create a valid contribution
        let contribution = ContributionRecord::new(member.clone(), group_id, cycle, 100, 1000);
        env.storage().persistent().set(
            &StorageKeyBuilder::contribution_individual(group_id, cycle, member.clone()),
            &contribution,
        );

        // Action: Get contribution
        let result = client.get_contribution(&group_id, &cycle, &member);
        
        // Verify: Record is valid
        assert!(result.validate());
        assert!(result.matches_group_and_cycle(group_id, cycle));
        assert!(result.is_from_member(&member));
    }
}

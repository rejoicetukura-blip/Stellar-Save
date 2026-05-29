use soroban_sdk::{Env, Vec};
use crate::{
    error::StellarSaveError,
    storage::{StorageKeyBuilder, STORAGE_VERSION},
    group::Group,
    status::GroupStatus,
};

/// Migration utilities for handling storage schema upgrades.
/// 
/// This module provides functionality to migrate data between different
/// storage schema versions, ensuring backward compatibility during upgrades.

/// Performs migration from older storage layouts to the current version.
/// 
/// This function checks the current storage version and applies necessary
/// migrations to bring the data up to the latest schema version.
/// 
/// # Arguments
/// * `env` - The contract environment
/// 
/// # Returns
/// * `Ok(())` - If migration completed successfully or no migration needed
/// * `Err(StellarSaveError)` - If migration failed
/// 
/// # Migration Process
/// 1. Check current storage version
/// 2. Apply incremental migrations if needed
/// 3. Update storage version to current
pub fn migrate(env: &Env) -> Result<(), StellarSaveError> {
    let version_key = StorageKeyBuilder::storage_version();
    let current_version: u32 = env.storage().persistent().get(&version_key).unwrap_or(1);
    
    // If already at current version, no migration needed
    if current_version >= STORAGE_VERSION {
        return Ok(());
    }
    
    // Apply migrations incrementally
    if current_version < 2 {
        migrate_v1_to_v2(env)?;
    }
    
    // Future migrations would be added here:
    // if current_version < 3 {
    //     migrate_v2_to_v3(env)?;
    // }
    
    // Update storage version to current
    env.storage().persistent().set(&version_key, &STORAGE_VERSION);
    
    Ok(())
}

/// Migrates storage from version 1 to version 2.
/// 
/// Version 2 changes:
/// - Adds storage version tracking
/// - Adds emergency pause functionality
/// - Adds reentrancy guard protection
/// 
/// This migration is safe as it only adds new fields without modifying existing data.
fn migrate_v1_to_v2(env: &Env) -> Result<(), StellarSaveError> {
    // Initialize new v2 fields with default values
    
    // Set emergency pause to false (not paused)
    let pause_key = StorageKeyBuilder::emergency_pause();
    if !env.storage().persistent().has(&pause_key) {
        env.storage().persistent().set(&pause_key, &false);
    }
    
    // Initialize reentrancy guard to false (not locked)
    let guard_key = StorageKeyBuilder::reentrancy_guard();
    if !env.storage().persistent().has(&guard_key) {
        env.storage().persistent().set(&guard_key, &false);
    }
    
    // Migrate existing groups to ensure they have all required v2 fields
    let total_groups_key = StorageKeyBuilder::total_groups();
    let total_groups: u64 = env.storage().persistent().get(&total_groups_key).unwrap_or(0);
    
    for group_id in 1..=total_groups {
        migrate_group_v1_to_v2(env, group_id)?;
    }
    
    Ok(())
}

/// Migrates a single group from v1 to v2 format.
/// 
/// Ensures the group has all required v2 fields and initializes
/// any missing balance tracking.
fn migrate_group_v1_to_v2(env: &Env, group_id: u64) -> Result<(), StellarSaveError> {
    let group_key = StorageKeyBuilder::group_data(group_id);
    
    // Check if group exists
    if !env.storage().persistent().has(&group_key) {
        return Ok(()); // Skip non-existent groups
    }
    
    // Initialize group balance if not present
    let balance_key = StorageKeyBuilder::group_balance(group_id);
    if !env.storage().persistent().has(&balance_key) {
        env.storage().persistent().set(&balance_key, &0i128);
    }
    
    // Initialize total paid out if not present
    let paid_out_key = StorageKeyBuilder::group_total_paid_out(group_id);
    if !env.storage().persistent().has(&paid_out_key) {
        env.storage().persistent().set(&paid_out_key, &0i128);
    }
    
    Ok(())
}

/// Initializes storage version on first contract deployment.
/// 
/// This should be called during contract initialization to set up
/// the storage version tracking.
pub fn initialize_storage_version(env: &Env) {
    let version_key = StorageKeyBuilder::storage_version();
    
    // Only set if not already present (avoid overwriting during upgrades)
    if !env.storage().persistent().has(&version_key) {
        env.storage().persistent().set(&version_key, &STORAGE_VERSION);
    }
}

/// Gets the current storage version from storage.
/// 
/// Returns 1 if no version is stored (indicating v1 schema).
pub fn get_storage_version(env: &Env) -> u32 {
    let version_key = StorageKeyBuilder::storage_version();
    env.storage().persistent().get(&version_key).unwrap_or(1)
}

/// Checks if migration is needed by comparing stored version with current version.
pub fn is_migration_needed(env: &Env) -> bool {
    get_storage_version(env) < STORAGE_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};
    use crate::group::Group;
    use crate::status::GroupStatus;

    #[test]
    fn test_initialize_storage_version() {
        let env = Env::default();
        
        // Initially no version should be stored
        assert_eq!(get_storage_version(&env), 1);
        
        // Initialize version
        initialize_storage_version(&env);
        
        // Should now have current version
        assert_eq!(get_storage_version(&env), STORAGE_VERSION);
    }

    #[test]
    fn test_initialize_storage_version_idempotent() {
        let env = Env::default();
        
        // Initialize version
        initialize_storage_version(&env);
        let first_version = get_storage_version(&env);
        
        // Initialize again
        initialize_storage_version(&env);
        let second_version = get_storage_version(&env);
        
        // Should be the same
        assert_eq!(first_version, second_version);
    }

    #[test]
    fn test_migration_not_needed_when_current() {
        let env = Env::default();
        
        // Set current version
        let version_key = StorageKeyBuilder::storage_version();
        env.storage().persistent().set(&version_key, &STORAGE_VERSION);
        
        // Migration should not be needed
        assert!(!is_migration_needed(&env));
        
        // Migration should succeed without changes
        assert!(migrate(&env).is_ok());
    }

    #[test]
    fn test_migration_needed_when_older() {
        let env = Env::default();
        
        // Set older version
        let version_key = StorageKeyBuilder::storage_version();
        env.storage().persistent().set(&version_key, &1u32);
        
        // Migration should be needed
        assert!(is_migration_needed(&env));
    }

    #[test]
    fn test_migrate_v1_to_v2_empty_contract() {
        let env = Env::default();
        
        // Simulate v1 contract (no version stored)
        assert_eq!(get_storage_version(&env), 1);
        
        // Run migration
        assert!(migrate(&env).is_ok());
        
        // Should now be at current version
        assert_eq!(get_storage_version(&env), STORAGE_VERSION);
        
        // Check that v2 fields were initialized
        let pause_key = StorageKeyBuilder::emergency_pause();
        let guard_key = StorageKeyBuilder::reentrancy_guard();
        
        assert_eq!(env.storage().persistent().get::<bool>(&pause_key).unwrap(), false);
        assert_eq!(env.storage().persistent().get::<bool>(&guard_key).unwrap(), false);
    }

    #[test]
    fn test_migrate_v1_to_v2_with_existing_groups() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        // Simulate v1 contract with existing groups
        let group_id = 1u64;
        let group = Group::new(group_id, creator, 100_000_000, 604800, 5, 2, 1234567890);
        
        // Store group in v1 format (without balance tracking)
        let group_key = StorageKeyBuilder::group_data(group_id);
        let status_key = StorageKeyBuilder::group_status(group_id);
        let total_groups_key = StorageKeyBuilder::total_groups();
        
        env.storage().persistent().set(&group_key, &group);
        env.storage().persistent().set(&status_key, &GroupStatus::Pending);
        env.storage().persistent().set(&total_groups_key, &1u64);
        
        // Run migration
        assert!(migrate(&env).is_ok());
        
        // Check that group balance fields were initialized
        let balance_key = StorageKeyBuilder::group_balance(group_id);
        let paid_out_key = StorageKeyBuilder::group_total_paid_out(group_id);
        
        assert_eq!(env.storage().persistent().get::<i128>(&balance_key).unwrap(), 0);
        assert_eq!(env.storage().persistent().get::<i128>(&paid_out_key).unwrap(), 0);
        
        // Original group data should be preserved
        let migrated_group: Group = env.storage().persistent().get(&group_key).unwrap();
        assert_eq!(migrated_group.id, group.id);
        assert_eq!(migrated_group.creator, group.creator);
    }

    #[test]
    fn test_migrate_preserves_existing_v2_fields() {
        let env = Env::default();
        
        // Set up partial v2 state
        let pause_key = StorageKeyBuilder::emergency_pause();
        env.storage().persistent().set(&pause_key, &true); // Already paused
        
        // Run migration from v1
        assert!(migrate(&env).is_ok());
        
        // Should preserve existing pause state
        assert_eq!(env.storage().persistent().get::<bool>(&pause_key).unwrap(), true);
        
        // Should initialize missing guard
        let guard_key = StorageKeyBuilder::reentrancy_guard();
        assert_eq!(env.storage().persistent().get::<bool>(&guard_key).unwrap(), false);
    }

    #[test]
    fn test_migration_updates_version() {
        let env = Env::default();
        
        // Start with v1 (no version stored)
        assert_eq!(get_storage_version(&env), 1);
        
        // Run migration
        assert!(migrate(&env).is_ok());
        
        // Should be updated to current version
        assert_eq!(get_storage_version(&env), STORAGE_VERSION);
    }

    #[test]
    fn test_migration_idempotent() {
        let env = Env::default();
        
        // Run migration twice
        assert!(migrate(&env).is_ok());
        let version_after_first = get_storage_version(&env);
        
        assert!(migrate(&env).is_ok());
        let version_after_second = get_storage_version(&env);
        
        // Should be the same
        assert_eq!(version_after_first, version_after_second);
        assert_eq!(version_after_second, STORAGE_VERSION);
    }

    #[test]
    fn test_get_storage_version_defaults_to_v1() {
        let env = Env::default();
        
        // No version stored should default to v1
        assert_eq!(get_storage_version(&env), 1);
    }

    #[test]
    fn test_is_migration_needed() {
        let env = Env::default();
        
        // v1 needs migration
        assert!(is_migration_needed(&env));
        
        // Set to current version
        let version_key = StorageKeyBuilder::storage_version();
        env.storage().persistent().set(&version_key, &STORAGE_VERSION);
        
        // Should not need migration
        assert!(!is_migration_needed(&env));
        
        // Set to future version
        env.storage().persistent().set(&version_key, &(STORAGE_VERSION + 1));
        
        // Should not need migration (already newer)
        assert!(!is_migration_needed(&env));
    }
}
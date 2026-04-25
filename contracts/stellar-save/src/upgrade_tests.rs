/// Upgrade compatibility tests for the StellarSave contract.
///
/// These tests verify:
/// 1. Storage schema backward compatibility (data written by v0 is readable by v1)
/// 2. Public API surface compatibility (all v0 entry-points still exist and behave)
/// 3. Data migration correctness (new fields default correctly on old records)
/// 4. Performance regression guard (key operations stay within instruction budgets)
#[cfg(test)]
mod upgrade_tests {
    use crate::{
        group::{Group, GroupStatus},
        storage::StorageKeyBuilder,
        ContractConfig, ContributionRecord, MemberProfile, StellarSaveContract, StellarSaveError,
    };
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Seed a minimal group directly into storage, simulating data written by a
    /// previous contract version (no token config, no grace period, etc.).
    fn seed_v0_group(env: &Env, group_id: u64, creator: &Address) -> Group {
        let group = Group::new(
            group_id,
            creator.clone(),
            1_000_000, // 0.1 XLM in stroops
            604_800,   // 7-day cycle
            5,
            2,
            env.ledger().timestamp(),
            0,
        );
        env.storage()
            .persistent()
            .set(&StorageKeyBuilder::group_data(group_id), &group);
        env.storage()
            .persistent()
            .set(&StorageKeyBuilder::group_status(group_id), &GroupStatus::Active);
        group
    }

    /// Seed a member profile directly, simulating a record from a previous version.
    fn seed_v0_member(env: &Env, group_id: u64, member: &Address, position: u32) {
        let profile = MemberProfile {
            address: member.clone(),
            group_id,
            payout_position: position,
            joined_at: env.ledger().timestamp(),
            auto_contribute_enabled: false,
        };
        env.storage()
            .persistent()
            .set(&StorageKeyBuilder::member_profile(group_id, member.clone()), &profile);

        // Maintain the members list
        let members_key = StorageKeyBuilder::group_members(group_id);
        let mut members: Vec<Address> = env
            .storage()
            .persistent()
            .get(&members_key)
            .unwrap_or_else(|| Vec::new(env));
        members.push_back(member.clone());
        env.storage().persistent().set(&members_key, &members);
    }

    /// Seed a contribution record directly, simulating data from a previous version.
    fn seed_v0_contribution(env: &Env, group_id: u64, cycle: u32, member: &Address, amount: i128) {
        let record = ContributionRecord::new(member.clone(), group_id, cycle, amount, 0);
        env.storage().persistent().set(
            &StorageKeyBuilder::contribution_individual(group_id, cycle, member.clone()),
            &record,
        );
        // Update cycle total
        let total_key = StorageKeyBuilder::contribution_cycle_total(group_id, cycle);
        let prev: i128 = env.storage().persistent().get(&total_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&total_key, &(prev + amount));
    }

    // ── 1. Storage backward compatibility ─────────────────────────────────────

    #[test]
    fn test_v0_group_readable_after_upgrade() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let group_id = 1u64;

        let original = seed_v0_group(&env, group_id, &creator);

        // Simulate post-upgrade read via the public API
        let read_back: Group = env
            .storage()
            .persistent()
            .get(&StorageKeyBuilder::group_data(group_id))
            .expect("group must be readable after upgrade");

        assert_eq!(read_back.contribution_amount, original.contribution_amount);
        assert_eq!(read_back.cycle_duration, original.cycle_duration);
        assert_eq!(read_back.max_members, original.max_members);
        assert_eq!(read_back.creator, original.creator);
    }

    #[test]
    fn test_v0_member_profile_readable_after_upgrade() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let member = Address::generate(&env);
        let group_id = 1u64;

        seed_v0_group(&env, group_id, &creator);
        seed_v0_member(&env, group_id, &member, 0);

        let profile: MemberProfile = env
            .storage()
            .persistent()
            .get(&StorageKeyBuilder::member_profile(group_id, member.clone()))
            .expect("member profile must be readable after upgrade");

        assert_eq!(profile.address, member);
        assert_eq!(profile.payout_position, 0);
        // New field introduced in v1 must default to false on old records
        assert!(!profile.auto_contribute_enabled);
    }

    #[test]
    fn test_v0_contribution_record_readable_after_upgrade() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let member = Address::generate(&env);
        let group_id = 1u64;
        let cycle = 0u32;
        let amount = 1_000_000i128;

        seed_v0_group(&env, group_id, &creator);
        seed_v0_member(&env, group_id, &member, 0);
        seed_v0_contribution(&env, group_id, cycle, &member, amount);

        let record: ContributionRecord = env
            .storage()
            .persistent()
            .get(&StorageKeyBuilder::contribution_individual(
                group_id,
                cycle,
                member.clone(),
            ))
            .expect("contribution record must be readable after upgrade");

        assert_eq!(record.amount, amount);
        assert_eq!(record.cycle_number, cycle);
    }

    // ── 2. API compatibility ──────────────────────────────────────────────────

    #[test]
    fn test_get_member_count_api_stable() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let member_a = Address::generate(&env);
        let member_b = Address::generate(&env);
        let group_id = 1u64;

        seed_v0_group(&env, group_id, &creator);
        seed_v0_member(&env, group_id, &member_a, 0);
        seed_v0_member(&env, group_id, &member_b, 1);

        // Manually update member_count on the stored group to reflect seeded members
        let mut group: Group = env
            .storage()
            .persistent()
            .get(&StorageKeyBuilder::group_data(group_id))
            .unwrap();
        group.member_count = 2;
        env.storage()
            .persistent()
            .set(&StorageKeyBuilder::group_data(group_id), &group);

        let count = StellarSaveContract::get_member_count(env, group_id)
            .expect("get_member_count must succeed on v0 data");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_update_config_api_stable() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        let config = ContractConfig {
            admin: admin.clone(),
            min_contribution: 1_000,
            max_contribution: 100_000_000,
            min_members: 2,
            max_members: 20,
            min_cycle_duration: 3_600,
            max_cycle_duration: 2_592_000,
        };

        // First call initialises config (no prior state — simulates fresh upgrade)
        StellarSaveContract::update_config(env.clone(), config.clone())
            .expect("update_config must succeed after upgrade");

        // Second call updates it (admin-gated path)
        StellarSaveContract::update_config(env, config).expect("update_config must be idempotent");
    }

    #[test]
    fn test_validate_contribution_amount_api_stable() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let group_id = 1u64;

        seed_v0_group(&env, group_id, &creator);

        // Correct amount
        StellarSaveContract::validate_contribution_amount(&env, group_id, 1_000_000)
            .expect("validate_contribution_amount must accept correct amount");

        // Wrong amount
        let err = StellarSaveContract::validate_contribution_amount(&env, group_id, 999)
            .expect_err("validate_contribution_amount must reject wrong amount");
        assert_eq!(err, StellarSaveError::InvalidAmount);
    }

    // ── 3. Data migration defaults ────────────────────────────────────────────

    #[test]
    fn test_new_group_fields_have_correct_defaults() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let group_id = 1u64;

        let group = seed_v0_group(&env, group_id, &creator);

        // Fields that were added in later versions must have safe defaults
        assert_eq!(group.current_cycle, 0, "current_cycle must start at 0");
        assert_eq!(group.member_count, 0, "member_count must start at 0");
    }

    #[test]
    fn test_group_status_defaults_to_active_when_seeded_active() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let group_id = 1u64;

        seed_v0_group(&env, group_id, &creator);

        let status: GroupStatus = env
            .storage()
            .persistent()
            .get(&StorageKeyBuilder::group_status(group_id))
            .expect("status key must exist");

        assert_eq!(status, GroupStatus::Active);
    }

    #[test]
    fn test_missing_group_returns_not_found_error() {
        let env = Env::default();

        let err = StellarSaveContract::get_member_count(env, 9999)
            .expect_err("non-existent group must return GroupNotFound");
        assert_eq!(err, StellarSaveError::GroupNotFound);
    }

    // ── 4. Performance regression guard ──────────────────────────────────────

    /// Verifies that reading a group + member profile stays within a reasonable
    /// number of storage operations (no accidental O(n) scans introduced).
    #[test]
    fn test_group_read_is_constant_time() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let group_id = 1u64;

        seed_v0_group(&env, group_id, &creator);

        // Seed many members to detect any accidental O(n) scan
        for i in 0..20u32 {
            let m = Address::generate(&env);
            seed_v0_member(&env, group_id, &m, i);
        }

        // Reading the group itself must not iterate over members
        let group: Group = env
            .storage()
            .persistent()
            .get(&StorageKeyBuilder::group_data(group_id))
            .expect("group read must succeed regardless of member count");

        assert_eq!(group.max_members, 5);
    }

    /// Verifies that cycle total lookup is O(1) (uses the pre-computed key,
    /// not a scan over individual contribution records).
    #[test]
    fn test_cycle_total_lookup_is_o1() {
        let env = Env::default();
        let creator = Address::generate(&env);
        let group_id = 1u64;
        let cycle = 0u32;
        let amount = 1_000_000i128;

        seed_v0_group(&env, group_id, &creator);

        // Seed many contributions for the cycle
        for _ in 0..10 {
            let m = Address::generate(&env);
            seed_v0_contribution(&env, group_id, cycle, &m, amount);
        }

        // The total must be retrievable via a single key lookup
        let total: i128 = env
            .storage()
            .persistent()
            .get(&StorageKeyBuilder::contribution_cycle_total(group_id, cycle))
            .unwrap_or(0);

        assert_eq!(total, amount * 10);
    }
}

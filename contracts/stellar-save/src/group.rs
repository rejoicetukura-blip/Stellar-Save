use soroban_sdk::{contracttype, Address, Env, Symbol};

/// Event emitted when a group is activated.
#[contracttype]
#[derive(Clone)]
pub struct GroupActivatedEvent {
    /// The group ID that was activated.
    pub group_id: u64,
    /// The timestamp when the group was activated.
    pub started_at: u64,
    /// The number of members in the group at activation.
    pub member_count: u32,
}

/// Emits a GroupActivatedEvent.
pub fn emit_group_activated(env: &Env, group_id: u64, started_at: u64, member_count: u32) {
    let topic = Symbol::new(env, "group_activated");
    env.events()
        .publish((topic,), GroupActivatedEvent {
            group_id,
            started_at,
            member_count,
        });
}

/// Core Group data structure representing a rotational savings group (ROSCA).
/// 
/// A Group manages the configuration and state of a savings circle where members
/// contribute a fixed amount each cycle and take turns receiving the pooled funds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    /// Unique identifier for the group.
    /// Generated sequentially when groups are created.
    pub id: u64,

    /// Address of the group creator.
    /// The creator has special privileges like starting the first cycle
    /// and potentially managing group settings.
    pub creator: Address,

    /// Fixed contribution amount in stroops (1 XLM = 10^7 stroops).
    /// All members must contribute this exact amount each cycle.
    /// Must be greater than 0.
    pub contribution_amount: i128,

    /// Duration of each cycle in seconds.
    /// Defines how long members have to contribute before payout.
    /// Common values: 604800 (1 week), 2592000 (30 days).
    /// Must be greater than 0.
    pub cycle_duration: u64,

    /// Maximum number of members allowed in the group.
    /// Once reached, no new members can join.
    /// Must be at least 2 (minimum for a meaningful ROSCA).
    /// Determines total number of cycles (one payout per member).
    pub max_members: u32,

    /// Minimum number of members required to activate the group.
    /// The group cannot start until this many members have joined.
    /// Must be at least 2 and not greater than max_members.
    pub min_members: u32,

    /// Current number of members in the group.
    /// Tracks how many members have joined.
    pub member_count: u32,

    /// Current cycle number (0-indexed).
    /// Increments after each successful payout.
    /// When current_cycle reaches max_members, the group is complete.
    pub current_cycle: u32,

    /// Whether the group is currently active and accepting contributions.
    /// Set to false when:
    /// - Group is paused by admin/creator
    /// - All cycles are complete
    /// - Group encounters an error state
    pub is_active: bool,

    /// Whether the group has been activated (started).
    /// Once activated, the group cannot accept new members.
    pub started: bool,

    /// Timestamp when the group was created (Unix timestamp in seconds).
    /// Used for tracking group age and calculating cycle deadlines.
    pub created_at: u64,

    /// Timestamp when the group was activated (Unix timestamp in seconds).
    /// Used for tracking when the first cycle started.
    /// Only set when started is true.
    pub started_at: u64,
}

impl Group {
    /// Creates a new Group with validation.
    /// 
    /// # Arguments
    /// * `id` - Unique group identifier
    /// * `creator` - Address of the group creator
    /// * `contribution_amount` - Amount each member contributes per cycle (in stroops)
    /// * `cycle_duration` - Duration of each cycle in seconds
    /// * `max_members` - Maximum number of members allowed
    /// * `min_members` - Minimum number of members required to activate the group
    /// * `created_at` - Creation timestamp
    /// 
    /// # Panics
    /// Panics if validation constraints are violated:
    /// - contribution_amount must be > 0
    /// - cycle_duration must be > 0
    /// - max_members must be >= 2
    /// - min_members must be >= 2
    /// - min_members must be <= max_members
    pub fn new(
        id: u64,
        creator: Address,
        contribution_amount: i128,
        cycle_duration: u64,
        max_members: u32,
        min_members: u32,
        created_at: u64,
    ) -> Self {
        // Validate contribution amount
        assert!(
            contribution_amount > 0,
            "contribution_amount must be greater than 0"
        );

        // Validate cycle duration
        assert!(
            cycle_duration > 0,
            "cycle_duration must be greater than 0"
        );

        // Validate max members (minimum 2 for a meaningful ROSCA)
        assert!(
            max_members >= 2,
            "max_members must be at least 2"
        );

        // Validate min members (minimum 2 for a meaningful ROSCA)
        assert!(
            min_members >= 2,
            "min_members must be at least 2"
        );

        // Validate min_members <= max_members
        assert!(
            min_members <= max_members,
            "min_members must be less than or equal to max_members"
        );

        Self {
            id,
            creator,
            contribution_amount,
            cycle_duration,
            max_members,
            min_members,
            member_count: 0,
            current_cycle: 0,
            is_active: true,
            started: false,
            created_at,
            started_at: 0,
        }
    }

    /// Checks if the group has completed all cycles.
    /// A group is complete when current_cycle equals max_members.
    pub fn is_complete(&self) -> bool {
        self.current_cycle >= self.max_members
    }

    /// Advances to the next cycle.
    /// Should be called after a successful payout.
    /// 
    /// # Panics
    /// Panics if the group is already complete.
    pub fn advance_cycle(&mut self) {
        assert!(!self.is_complete(), "group is already complete");
        self.current_cycle += 1;
        
        // Deactivate if we've reached the final cycle
        if self.is_complete() {
            self.is_active = false;
        }
    }

    /// Deactivates the group, preventing further contributions.
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Reactivates the group if it's not complete.
    /// 
    /// # Panics
    /// Panics if attempting to reactivate a completed group.
    pub fn reactivate(&mut self) {
        assert!(!self.is_complete(), "cannot reactivate a completed group");
        self.is_active = true;
    }

    /// Activates the group (starts the first cycle) once minimum members have joined.
    /// 
    /// # Arguments
    /// * `timestamp` - Current timestamp when activation occurs
    /// 
    /// # Panics
    /// Panics if:
    /// - Group has already been started
    /// - Minimum member count has not been reached
    pub fn activate(&mut self, timestamp: u64) {
        // Check if already started
        assert!(!self.started, "group has already been started");
        
        // Check if minimum members have joined
        assert!(
            self.member_count >= self.min_members,
            "minimum members ({}) required to activate, currently have {}",
            self.min_members,
            self.member_count
        );
        
        self.started = true;
        self.started_at = timestamp;
    }

    /// Checks if the group has met the minimum member requirement for activation.
    pub fn can_activate(&self) -> bool {
        !self.started && self.member_count >= self.min_members
    }

    /// Calculates the total pool amount for a cycle.
    /// This is the amount distributed to the recipient each cycle.
    pub fn total_pool_amount(&self) -> i128 {
        self.contribution_amount * (self.max_members as i128)
    }

    /// Validates that the group configuration is sound.
    /// Returns true if all constraints are met.
    pub fn validate(&self) -> bool {
        self.contribution_amount > 0
            && self.cycle_duration > 0
            && self.max_members >= 2
            && self.min_members >= 2
            && self.min_members <= self.max_members
            && self.current_cycle <= self.max_members
    }

    /// Adds a member to the group.
    /// 
    /// # Panics
    /// Panics if:
    /// - Group has already started
    /// - Group has reached maximum members
    pub fn add_member(&mut self) {
        assert!(!self.started, "cannot add members after group has started");
        assert!(
            self.member_count < self.max_members,
            "group has reached maximum members"
        );
        self.member_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_group_creation() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let group = Group::new(
            1,
            creator.clone(),
            10_000_000, // 1 XLM
            604800,     // 1 week
            5,          // 5 members
            2,          // 2 min members
            1234567890,
        );

        assert_eq!(group.id, 1);
        assert_eq!(group.creator, creator);
        assert_eq!(group.contribution_amount, 10_000_000);
        assert_eq!(group.cycle_duration, 604800);
        assert_eq!(group.max_members, 5);
        assert_eq!(group.min_members, 2);
        assert_eq!(group.member_count, 0);
        assert_eq!(group.current_cycle, 0);
        assert_eq!(group.is_active, true);
        assert_eq!(group.started, false);
        assert_eq!(group.created_at, 1234567890);
    }

    #[test]
    #[should_panic(expected = "min_members must be at least 2")]
    fn test_invalid_min_members() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        Group::new(1, creator, 10_000_000, 604800, 5, 1, 1234567890);
    }

    #[test]
    #[should_panic(expected = "min_members must be less than or equal to max_members")]
    fn test_min_members_greater_than_max() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        Group::new(1, creator, 10_000_000, 604800, 3, 5, 1234567890);
    }

    #[test]
    #[should_panic(expected = "contribution_amount must be greater than 0")]
    fn test_invalid_contribution_amount() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        Group::new(1, creator, 0, 604800, 5, 2, 1234567890);
    }

    #[test]
    #[should_panic(expected = "cycle_duration must be greater than 0")]
    fn test_invalid_cycle_duration() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        Group::new(1, creator, 10_000_000, 0, 5, 2, 1234567890);
    }

    #[test]
    #[should_panic(expected = "max_members must be at least 2")]
    fn test_invalid_max_members() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        Group::new(1, creator, 10_000_000, 604800, 1, 2, 1234567890);
    }

    #[test]
    fn test_is_complete() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 3, 2, 1234567890);
        
        assert!(!group.is_complete());
        
        group.current_cycle = 2;
        assert!(!group.is_complete());
        
        group.current_cycle = 3;
        assert!(group.is_complete());
    }

    #[test]
    fn test_advance_cycle() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 3, 2, 1234567890);
        
        assert_eq!(group.current_cycle, 0);
        assert!(group.is_active);
        
        group.advance_cycle();
        assert_eq!(group.current_cycle, 1);
        assert!(group.is_active);
        
        group.advance_cycle();
        assert_eq!(group.current_cycle, 2);
        assert!(group.is_active);
        
        group.advance_cycle();
        assert_eq!(group.current_cycle, 3);
        assert!(!group.is_active); // Auto-deactivated when complete
    }

    #[test]
    #[should_panic(expected = "group is already complete")]
    fn test_advance_cycle_when_complete() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 2, 2, 1234567890);
        group.current_cycle = 2;
        
        group.advance_cycle(); // Should panic
    }

    #[test]
    fn test_deactivate_reactivate() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 3, 2, 1234567890);
        
        assert!(group.is_active);
        
        group.deactivate();
        assert!(!group.is_active);
        
        group.reactivate();
        assert!(group.is_active);
    }

    #[test]
    #[should_panic(expected = "cannot reactivate a completed group")]
    fn test_reactivate_completed_group() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 2, 2, 1234567890);
        group.current_cycle = 2;
        
        group.reactivate(); // Should panic
    }

    #[test]
    fn test_total_pool_amount() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let group = Group::new(1, creator, 10_000_000, 604800, 5, 2, 1234567890);
        
        assert_eq!(group.total_pool_amount(), 50_000_000); // 5 XLM total
    }

    #[test]
    fn test_validate() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let group = Group::new(1, creator, 10_000_000, 604800, 5, 2, 1234567890);
        assert!(group.validate());
    }

    #[test]
    fn test_activate_group() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 5, 2, 1234567890);
        
        // Initially not started
        assert!(!group.started);
        assert_eq!(group.started_at, 0);
        
        // Cannot activate with less than min_members
        assert!(!group.can_activate());
        
        // Add members
        group.add_member();
        assert_eq!(group.member_count, 1);
        assert!(!group.can_activate());
        
        group.add_member();
        assert_eq!(group.member_count, 2);
        assert!(group.can_activate());
        
        // Activate the group
        group.activate(1234568000);
        
        assert!(group.started);
        assert_eq!(group.started_at, 1234568000);
    }

    #[test]
    #[should_panic(expected = "group has already been started")]
    fn test_activate_already_started() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 5, 2, 1234567890);
        
        group.add_member();
        group.add_member();
        group.activate(1234568000);
        
        // Try to activate again - should panic
        group.activate(1234568001);
    }

    #[test]
    #[should_panic(expected = "minimum members")]
    fn test_activate_not_enough_members() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 5, 3, 1234567890);
        
        group.add_member();
        group.add_member();
        
        // Only 2 members, need 3 - should panic
        group.activate(1234568000);
    }

    #[test]
    #[should_panic(expected = "cannot add members after group has started")]
    fn test_add_member_after_start() {
        let env = Env::default();
        let creator = Address::generate(&env);
        
        let mut group = Group::new(1, creator, 10_000_000, 604800, 5, 2, 1234567890);
        
        group.add_member();
        group.add_member();
        group.activate(1234568000);
        
        // Try to add another member - should panic
        group.add_member();
    }
}

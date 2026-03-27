#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Balance(Address),
    Admin,
    Vesting(Address),
}

#[contracttype]
#[derive(Clone)]
pub struct VestingSchedule {
    pub beneficiary: Address,
    pub total_amount: i128,
    pub start_ledger: u32,
    pub cliff_ledger: u32,
    pub end_ledger: u32,
    pub claimed: i128,
}

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Mint reward tokens to a student upon course completion
    pub fn mint_reward(env: Env, caller: Address, recipient: Address, amount: i128) {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        assert!(caller == admin, "Only admin can mint");
        assert!(amount > 0, "Amount must be positive");

        Self::add_balance(&env, &recipient, amount);
    }

    pub fn balance(env: Env, addr: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(addr))
            .unwrap_or(0)
    }

    // -------------------------------------------------------------------------
    // Vesting
    // -------------------------------------------------------------------------

    /// Create a vesting schedule for an instructor (admin only).
    /// `start_ledger` is recorded as the current ledger at creation time.
    pub fn create_vesting(
        env: Env,
        admin: Address,
        beneficiary: Address,
        total_amount: i128,
        cliff_ledger: u32,
        end_ledger: u32,
    ) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        assert!(admin == stored_admin, "Only admin can create vesting");
        assert!(total_amount > 0, "Amount must be positive");

        let start_ledger = env.ledger().sequence();
        assert!(cliff_ledger >= start_ledger, "Cliff must be >= start");
        assert!(end_ledger > cliff_ledger, "End must be after cliff");

        // Overwrite any existing schedule for this beneficiary
        let schedule = VestingSchedule {
            beneficiary: beneficiary.clone(),
            total_amount,
            start_ledger,
            cliff_ledger,
            end_ledger,
            claimed: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Vesting(beneficiary), &schedule);
    }

    /// Claim vested tokens. Calculates the vested amount minus already claimed,
    /// mints the difference to the beneficiary, and updates `claimed`.
    pub fn claim_vesting(env: Env, beneficiary: Address) {
        beneficiary.require_auth();

        let key = DataKey::Vesting(beneficiary.clone());
        let mut schedule: VestingSchedule = env
            .storage()
            .persistent()
            .get(&key)
            .expect("No vesting schedule found");

        let current_ledger = env.ledger().sequence();
        let claimable = Self::vested_amount(&schedule, current_ledger) - schedule.claimed;
        assert!(claimable > 0, "Nothing to claim yet");

        schedule.claimed += claimable;
        env.storage().persistent().set(&key, &schedule);

        Self::add_balance(&env, &beneficiary, claimable);
    }

    /// Returns the vesting schedule for a beneficiary.
    pub fn get_vesting(env: Env, beneficiary: Address) -> Option<VestingSchedule> {
        env.storage()
            .persistent()
            .get(&DataKey::Vesting(beneficiary))
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn vested_amount(schedule: &VestingSchedule, current_ledger: u32) -> i128 {
        if current_ledger < schedule.cliff_ledger {
            return 0;
        }
        if current_ledger >= schedule.end_ledger {
            return schedule.total_amount;
        }
        // Linear vesting between start and end
        let elapsed = (current_ledger - schedule.start_ledger) as i128;
        let duration = (schedule.end_ledger - schedule.start_ledger) as i128;
        schedule.total_amount * elapsed / duration
    }

    fn add_balance(env: &Env, addr: &Address, amount: i128) {
        let current: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(addr.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(addr.clone()), &(current + amount));
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::Env;

    fn setup() -> (Env, TokenContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, TokenContract);
        let client = TokenContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let instructor = Address::generate(&env);

        client.initialize(&admin);
        (env, client, admin, instructor)
    }

    fn set_ledger(env: &Env, sequence: u32) {
        env.ledger().set(LedgerInfo {
            sequence_number: sequence,
            timestamp: sequence as u64 * 5,
            protocol_version: 21,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1000,
            min_persistent_entry_ttl: 1000,
            max_entry_ttl: 100_000,
        });
    }

    #[test]
    #[should_panic(expected = "Nothing to claim yet")]
    fn test_cliff_not_reached() {
        let (env, client, admin, instructor) = setup();
        // start=10, cliff=20, end=30
        set_ledger(&env, 10);
        client.create_vesting(&admin, &instructor, &1000, &20, &30);

        // At ledger 15 (before cliff) — nothing claimable, should panic
        set_ledger(&env, 15);
        client.claim_vesting(&instructor);
    }

    #[test]
    fn test_partial_vest() {
        let (env, client, admin, instructor) = setup();
        // start=10, cliff=10, end=30 → 20 ledger duration
        set_ledger(&env, 10);
        client.create_vesting(&admin, &instructor, &1000, &10, &30);

        // At ledger 20: elapsed=10, duration=20 → 500 vested
        set_ledger(&env, 20);
        client.claim_vesting(&instructor);

        assert_eq!(client.balance(&instructor), 500);
        let schedule = client.get_vesting(&instructor).unwrap();
        assert_eq!(schedule.claimed, 500);
    }

    #[test]
    fn test_full_vest() {
        let (env, client, admin, instructor) = setup();
        set_ledger(&env, 10);
        client.create_vesting(&admin, &instructor, &1000, &10, &30);

        // At or past end_ledger → full amount
        set_ledger(&env, 30);
        client.claim_vesting(&instructor);

        assert_eq!(client.balance(&instructor), 1000);
        let schedule = client.get_vesting(&instructor).unwrap();
        assert_eq!(schedule.claimed, 1000);
    }

    #[test]
    fn test_incremental_claims() {
        let (env, client, admin, instructor) = setup();
        set_ledger(&env, 0);
        client.create_vesting(&admin, &instructor, &1000, &0, &100);

        // Claim at 50% vested
        set_ledger(&env, 50);
        client.claim_vesting(&instructor);
        assert_eq!(client.balance(&instructor), 500);

        // Claim the rest at 100%
        set_ledger(&env, 100);
        client.claim_vesting(&instructor);
        assert_eq!(client.balance(&instructor), 1000);
    }

    #[test]
    #[should_panic(expected = "Only admin can create vesting")]
    fn test_only_admin_can_create_vesting() {
        let (env, client, _admin, instructor) = setup();
        set_ledger(&env, 10);
        let rando = Address::generate(&env);
        client.create_vesting(&rando, &instructor, &1000, &20, &30);
    }
}

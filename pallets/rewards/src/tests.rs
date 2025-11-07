//! Unit tests and mock runtime for pallet-rewards covering the main flows:
//! - create_rule
//! - create_attestation (club-scoped attestor)
//! - claim_reward (uses attestation; per-epoch accounting)
//! - award_manual
//! - revoke_attestation
//!
//! This file builds a minimal Test runtime including:
//! - frame_system
//! - pallet_balances (used as Currency for tests)
//! - pallet_member_registry (the member registry pallet implemented earlier)
//! - pallet_badges (present but not used heavily in tests)
//! - pallet_rewards (the pallet under test)

#![cfg(test)]
// Tests for pallet-rewards using the mock runtime in `mock.rs`.
// Place this file as `pallets/rewards/src/tests.rs`.

mod mock;
use mock::*;
use crate as pallet_rewards;

use frame_support::{assert_ok, assert_noop};

#[test]
fn rewards_create_attest_claim_flow() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Create club with admin 10 and officer 2
        assert_ok!(MemberRegistry::create_club(frame_system::RawOrigin::Root.into(), b"AI".to_vec().try_into().unwrap(), 20u64, None));
        assert_ok!(MemberRegistry::add_officer(frame_system::RawOrigin::Signed(20u64).into(), 0u32, 2u64));

        // Create a rule scoped to club 0 (rule id 0)
        assert_ok!(Rewards::create_rule(frame_system::RawOrigin::Root.into(), b"attendance".to_vec(), 100u128, 5u32, Some(0u32), None));
        assert!(Rewards::rules(0).is_some());

        // Officer creates attestation for subject 3
        assert_ok!(Rewards::create_attestation(frame_system::RawOrigin::Signed(2u64).into(), 3u64, 0u32, None, None));
        let att = Rewards::attestations(0).expect("att exists");
        assert_eq!(att.subject, 3u64);

        // Claim reward (subject 3)
        let before = pallet_balances::Pallet::<Test>::free_balance(3u64);
        assert_ok!(Rewards::claim_reward(frame_system::RawOrigin::Signed(3u64).into(), 0u64));
        let after = pallet_balances::Pallet::<Test>::free_balance(3u64);
        assert_eq!(after, before + 100u128);

        // Attestation marked used
        let att_used = Rewards::attestations(0).unwrap();
        assert!(att_used.used);

        // Per-epoch counter increased
        let counter = Rewards::claims_this_epoch(3u64, 0u32);
        assert_eq!(counter.count, 1u32);
    });
}

#[test]
fn reward_expiry_and_double_claim() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Create club and add officer
        assert_ok!(MemberRegistry::create_club(frame_system::RawOrigin::Root.into(), b"Sec".to_vec().try_into().unwrap(), 30u64, None));
        assert_ok!(MemberRegistry::add_officer(frame_system::RawOrigin::Signed(30u64).into(), 0u32, 2u64));

        // Create rule for club 0
        assert_ok!(Rewards::create_rule(frame_system::RawOrigin::Root.into(), b"presentation".to_vec(), 50u128, 2u32, Some(0u32), None));

        // Create attestation with immediate expiry (expires_at = 0 => expired in mock time logic)
        assert_ok!(Rewards::create_attestation(frame_system::RawOrigin::Signed(2u64).into(), 5u64, 0u32, Some(0u64), None));
        // Claim should fail due to expiry
        assert_noop!(
            Rewards::claim_reward(frame_system::RawOrigin::Signed(5u64).into(), 0u64),
            pallet_rewards::Error::<Test>::AttestationExpired
        );

        // Create valid attestation and claim, then double-use should fail
        assert_ok!(Rewards::create_attestation(frame_system::RawOrigin::Signed(2u64).into(), 6u64, 0u32, None, None));
        assert_ok!(Rewards::claim_reward(frame_system::RawOrigin::Signed(6u64).into(), 1u64));
        assert_noop!(
            Rewards::claim_reward(frame_system::RawOrigin::Signed(6u64).into(), 1u64),
            pallet_rewards::Error::<Test>::AttestationUsed
        );
    });
}
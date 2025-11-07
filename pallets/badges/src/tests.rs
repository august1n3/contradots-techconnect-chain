//! Unit tests and mock runtime for pallet-badges:
//! - create_class
//! - issue_badge (by creator & by club officer when club-scoped)
//! - transfer_badge with transferable & soulbound checks
//! - revoke_badge

#![cfg(test)]
// Tests for pallet-badges using the mock runtime in `mock.rs` (mock::*).
// Place as `pallets/badges/src/tests.rs`.

mod mock;
use mock::*;
use crate as pallet_badges;

use frame_support::{assert_ok, assert_noop};

#[test]
fn badges_create_issue_transfer_revoke() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Create a global class as Root (creator = 0)
        assert_ok!(Badges::create_class(
            frame_system::RawOrigin::Root.into(),
            [1u8; 32],
            None,
            false,
            true
        ));

        // Issue a transferable badge instance to account 1 by creator (0)
        assert_ok!(Badges::issue_badge(
            frame_system::RawOrigin::Signed(0u64).into(),
            0u32,
            1u64,
            [2u8; 32],
            Some(true),
            Some(false)
        ));

        // Owner is account 1
        let owner = Badges::owner_of(0u32, 0u64).expect("exists");
        assert_eq!(owner, 1u64);

        // Transfer from 1 -> 2
        assert_ok!(Badges::transfer_badge(frame_system::RawOrigin::Signed(1u64).into(), 0u32, 0u64, 2u64));
        let owner2 = Badges::owner_of(0u32, 0u64).unwrap();
        assert_eq!(owner2, 2u64);

        // Revoke by creator
        assert_ok!(Badges::revoke_badge(frame_system::RawOrigin::Signed(0u64).into(), 0u32, 0u64));
        assert!(Badges::instance_metadata(0u32, 0u64).is_none());
    });
}

#[test]
fn club_scoped_issue_and_soulbound_behavior() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Create club 0 with admin 10 and add officer 2
        assert_ok!(MemberRegistry::create_club(frame_system::RawOrigin::Root.into(), b"Club".to_vec().try_into().unwrap(), 10u64, None));
        assert_ok!(MemberRegistry::add_officer(frame_system::RawOrigin::Signed(10u64).into(), 0u32, 2u64));

        // Create a club-scoped class (club = 0)
        assert_ok!(Badges::create_class(
            frame_system::RawOrigin::Root.into(),
            [3u8; 32],
            Some(0u32),
            false,
            false
        ));

        // Non-officer tries to issue -> should fail
        assert_noop!(
            Badges::issue_badge(frame_system::RawOrigin::Signed(5u64).into(), 1u32, 4u64, [4u8;32], None, None),
            pallet_badges::Error::<Test>::NotIssuer
        );

        // Officer issues a soulbound badge to account 4
        assert_ok!(Badges::issue_badge(frame_system::RawOrigin::Signed(2u64).into(), 1u32, 4u64, [5u8;32], Some(false), Some(true)));

        // Owner cannot transfer soulbound badge
        assert_noop!(
            Badges::transfer_badge(frame_system::RawOrigin::Signed(4u64).into(), 1u32, 0u64, 3u64),
            pallet_badges::Error::<Test>::Soulbound
        );
    });
}
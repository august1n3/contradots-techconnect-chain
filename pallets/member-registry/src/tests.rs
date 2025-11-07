//! Unit tests and mock runtime for pallet-member-registry:
//! - create_club
//! - add_officer / remove_officer
//! - create_attestation / revoke_attestation
//! - register_member using attestation
//! - add_member_admin / remove_member_admin

#![cfg(test)]
// Tests for pallet-member-registry using the mock runtime in `mock.rs` (mock::*).
// Drop this file into `pallets/member-registry/src/tests.rs`.

mod mock;
use mock::*;
use crate as pallet_member_registry;

use frame_support::{assert_ok, assert_noop};

#[test]
fn member_registry_core_flows() {
    new_test_ext().execute_with(|| {
        // ensure block is initialized
        System::set_block_number(1);

        // 1) Create a club (Root origin)
        let club_admin: AccountId = 10;
        assert_ok!(MemberRegistry::create_club(
            frame_system::RawOrigin::Root.into(),
            b"Developers".to_vec().try_into().unwrap(),
            club_admin,
            None
        ));

        // 2) Add an officer (signed by club admin)
        assert_ok!(MemberRegistry::add_officer(
            frame_system::RawOrigin::Signed(club_admin).into(),
            0u32,
            2u64
        ));

        // 3) Officer creates an attestation for subject 3
        assert_ok!(MemberRegistry::create_attestation(
            frame_system::RawOrigin::Signed(2u64).into(),
            3u64,
            0u32,
            None,
            None
        ));

        // attestation record should exist
        let att = MemberRegistry::attestations(0).expect("attestation exists");
        assert_eq!(att.subject, 3u64);

        // 4) Subject uses attestation to register
        assert_ok!(MemberRegistry::register_member(
            frame_system::RawOrigin::Signed(3u64).into(),
            0u64
        ));

        // member record exists
        assert!(MemberRegistry::members(3u64).is_some());

        // 5) Admin directly adds a member (4)
        assert_ok!(MemberRegistry::add_member_admin(
            frame_system::RawOrigin::Signed(club_admin).into(),
            4u64,
            0u32
        ));
        assert!(MemberRegistry::members(4u64).is_some());

        // 6) Remove member (set status removed)
        assert_ok!(MemberRegistry::remove_member_admin(
            frame_system::RawOrigin::Signed(club_admin).into(),
            4u64,
            0u32
        ));
        let m = MemberRegistry::members(4u64).unwrap();
        assert_eq!(m.status, pallet_member_registry::MemberStatus::Removed);

        // 7) Attestor revokes attestation (attestor or admin)
        assert_ok!(MemberRegistry::revoke_attestation(
            frame_system::RawOrigin::Signed(2u64).into(),
            0u64
        ));
        assert!(MemberRegistry::attestations(0u64).is_none());
    });
}

#[test]
fn attestation_invalid_and_double_use() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Create club with admin 20 and officer 2
        assert_ok!(MemberRegistry::create_club(frame_system::RawOrigin::Root.into(), b"ClubX".to_vec().try_into().unwrap(), 20u64, None));
        assert_ok!(MemberRegistry::add_officer(frame_system::RawOrigin::Signed(20u64).into(), 0u32, 2u64));

        // Officer creates attestation for 5
        assert_ok!(MemberRegistry::create_attestation(frame_system::RawOrigin::Signed(2u64).into(), 5u64, 0u32, None, None));
        // 5 registers using attestation
        assert_ok!(MemberRegistry::register_member(frame_system::RawOrigin::Signed(5u64).into(), 0u64));

        // second registration attempt should fail (AlreadyMember)
        assert_noop!(
            MemberRegistry::register_member(frame_system::RawOrigin::Signed(5u64).into(), 0u64),
            pallet_member_registry::Error::<Test>::AlreadyMember
        );

        // non-officer (account 3) attempts to create attestation -> should fail
        assert_noop!(
            MemberRegistry::create_attestation(frame_system::RawOrigin::Signed(3u64).into(), 6u64, 0u32, None, None),
            pallet_member_registry::Error::<Test>::AttestorNotAuthorized
        );
    });
}
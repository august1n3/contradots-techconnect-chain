//! Unit tests for pallet-member-registry using the per-pallet mock runtime in `mock.rs`.
//! Place this file as `pallets/member-registry/src/tests.rs`.

#![cfg(test)]



use crate::{self as pallet_member_registry, mock::*};
use frame_support::pallet_prelude::ConstU32;
use frame_support::{assert_ok, assert_noop};
use sp_std::convert::TryInto;

#[test]
fn member_registry_core_flows() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // 1) Create a club (Root origin)
        let club_admin: AccountId = 10;
        let name_vec = b"Developers".to_vec();
        let name = frame_support::BoundedVec::<u8, ConstU32<64>>::try_from(name_vec).unwrap();

        assert_ok!(MemberRegistry::create_club(
            frame_system::RawOrigin::Root.into(),
            name,
            club_admin,
            None
        ));

        // club id should be 0 initially
        let club = MemberRegistry::clubs(0).expect("club exists");
        assert_eq!(club.admin, club_admin);

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

        // attestation record should exist (id 0)
        let att = MemberRegistry::attestations(0).expect("attestation exists");
        assert_eq!(att.subject, 3u64);
        assert_eq!(att.club, 0u32);

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
        // create another attestation by officer 2 for subject 5
        assert_ok!(MemberRegistry::create_attestation(
            frame_system::RawOrigin::Signed(2u64).into(),
            5u64,
            0u32,
            None,
            None
        ));
        // revoke it by attestor
        assert_ok!(MemberRegistry::revoke_attestation(
            frame_system::RawOrigin::Signed(2u64).into(),
            1u64
        ));
        assert!(MemberRegistry::attestations(1u64).is_none());
    });
}

#[test]
fn attestation_invalid_and_double_use() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Create club with admin 20 and officer 2
        let name_vec = b"ClubX".to_vec();
        let name = frame_support::BoundedVec::<u8, ConstU32<64>>::try_from(name_vec).unwrap();

        assert_ok!(MemberRegistry::create_club(frame_system::RawOrigin::Root.into(), name, 20u64, None));
        assert_ok!(MemberRegistry::add_officer(frame_system::RawOrigin::Signed(20u64).into(), 0u32, 2u64));

        // Officer creates attestation for 5
        assert_ok!(MemberRegistry::create_attestation(frame_system::RawOrigin::Signed(2u64).into(), 5u64, 0u32, None, None));
        // 5 registers using attestation
        assert_ok!(MemberRegistry::register_member(frame_system::RawOrigin::Signed(5u64).into(), 0u64));

        // second registration attempt should fail (AttestationUsed)
        assert_noop!(
            MemberRegistry::register_member(frame_system::RawOrigin::Signed(5u64).into(), 0u64),
            pallet_member_registry::Error::<Test>::AttestationUsed
        );

        // non-officer (account 3) attempts to create attestation -> should fail
        assert_noop!(
            MemberRegistry::create_attestation(frame_system::RawOrigin::Signed(3u64).into(), 6u64, 0u32, None, None),
            pallet_member_registry::Error::<Test>::AttestorNotAuthorized
        );
    });
}
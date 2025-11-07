//! Unit tests and mock runtime for pallet-proposal-router:
//! - propose (club & global eligibility checks)
//! - vote (prevent double votes, enforce membership requirements)
//! - execute: demonstrate executing a balances::transfer call encoded as RuntimeCall
//!
//! Note: the ProposalRouter::execute dispatches the stored call with Signed(pallet_account).
//! The pallet account must have funds for balances::transfer to succeed; we seed it in genesis.

#![cfg(test)]
// Tests for pallet-proposal-router using the mock runtime in `mock.rs`.
// Place as `pallets/proposal-router/src/tests.rs`.

mod mock;
use mock::*;
use crate as pallet_proposal_router;

use frame_support::{assert_ok, assert_noop};
use codec::Encode;

#[test]
fn propose_vote_execute_transfer() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Ensure the proposal pallet account has funds (seeded in mock)
        let pallet_acc = proposal_pallet_account();
        assert!(pallet_balances::Pallet::<Test>::free_balance(pallet_acc) > 0);

        // Create club and register a member 3 via attestation flow
        assert_ok!(MemberRegistry::create_club(frame_system::RawOrigin::Root.into(), b"Gov".to_vec().try_into().unwrap(), 10u64, None));
        assert_ok!(MemberRegistry::add_officer(frame_system::RawOrigin::Signed(10u64).into(), 0u32, 2u64));
        assert_ok!(MemberRegistry::create_attestation(frame_system::RawOrigin::Signed(2u64).into(), 3u64, 0u32, None, None));
        assert_ok!(MemberRegistry::register_member(frame_system::RawOrigin::Signed(3u64).into(), 0u64));

        // Prepare a balances::transfer call encoded
        let transfer_call = Call::Balances(pallet_balances::Call::transfer { dest: 1u64.into(), value: 1_000u128 });
        let encoded = transfer_call.encode();

        // Member 3 submits a global proposal
        assert_ok!(ProposalRouter::propose(frame_system::RawOrigin::Signed(3u64).into(), pallet_proposal_router::Scope::Global, encoded.clone(), None, None, None, None));

        // Vote by member 3
        assert_ok!(ProposalRouter::vote(frame_system::RawOrigin::Signed(3u64).into(), 0u64, true));

        // Advance blocks beyond voting end
        System::set_block_number(100);

        // Execute the proposal
        let before = pallet_balances::Pallet::<Test>::free_balance(1u64);
        assert_ok!(ProposalRouter::execute(frame_system::RawOrigin::Signed(1u64).into(), 0u64));
        let after = pallet_balances::Pallet::<Test>::free_balance(1u64);
        assert_eq!(after, before + 1_000u128);
    });
}

#[test]
fn club_proposal_eligibility_and_double_vote_prevention() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Create club 0 and member 3
        assert_ok!(MemberRegistry::create_club(frame_system::RawOrigin::Root.into(), b"Club".to_vec().try_into().unwrap(), 10u64, None));
        assert_ok!(MemberRegistry::add_officer(frame_system::RawOrigin::Signed(10u64).into(), 0u32, 2u64));
        assert_ok!(MemberRegistry::create_attestation(frame_system::RawOrigin::Signed(2u64).into(), 3u64, 0u32, None, None));
        assert_ok!(MemberRegistry::register_member(frame_system::RawOrigin::Signed(3u64).into(), 0u64));

        // Dummy call
        let transfer_call = Call::Balances(pallet_balances::Call::transfer { dest: 1u64.into(), value: 100u128 });
        let encoded = transfer_call.encode();

        // Non-member 5 cannot propose club-scoped
        assert_noop!(
            ProposalRouter::propose(frame_system::RawOrigin::Signed(5u64).into(), pallet_proposal_router::Scope::Club(0u32), encoded.clone(), None, None, None, None),
            pallet_proposal_router::Error::<Test>::NotClubMember
        );

        // Member 3 proposes OK
        assert_ok!(ProposalRouter::propose(frame_system::RawOrigin::Signed(3u64).into(), pallet_proposal_router::Scope::Club(0u32), encoded.clone(), None, None, None, None));

        // Member votes once and second vote fails
        assert_ok!(ProposalRouter::vote(frame_system::RawOrigin::Signed(3u64).into(), 0u64, true));
        assert_noop!(ProposalRouter::vote(frame_system::RawOrigin::Signed(3u64).into(), 0u64, true), pallet_proposal_router::Error::<Test>::AlreadyVoted);
    });
}
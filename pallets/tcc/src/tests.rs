//! Unit tests and mock runtime for pallet-tcc (asset wrapper):
//! - instantiate_asset
//! - mint (privileged origin) and unauthorized mint attempts
//! - transfer (user-signed)
//! - burn (privileged origin)
//!
//! NOTE: tests rely on pallet-assets being available in the runtime.

#![cfg(test)]
// Tests for pallet-tcc (asset wrapper) using the mock runtime in `mock.rs`.
// Place this file as `pallets/tcc/src/tests.rs`.

mod mock;
use mock::*;
use crate as pallet_tcc;

use frame_support::{assert_ok, assert_noop};

#[test]
fn tcc_instantiate_mint_transfer_burn() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // instantiate asset id 1000 with owner 99
        assert_ok!(Tcc::instantiate_asset(frame_system::RawOrigin::Root.into(), 99u64.into(), 1u128, false));
        assert!(Tcc::asset_exists());

        // Mint 1000 to account 1 by Root
        assert_ok!(Tcc::mint(frame_system::RawOrigin::Root.into(), 1u64.into(), 1000u128));

        // Check balance via pallet-assets API
        let bal = pallet_assets::Pallet::<Test>::balance(1000u32, &1u64);
        assert_eq!(bal, 1000u128);

        // Transfer 250 from 1 -> 2 using Tcc::transfer
        assert_ok!(Tcc::transfer(frame_system::RawOrigin::Signed(1u64).into(), 2u64.into(), 250u128));
        let bal1 = pallet_assets::Pallet::<Test>::balance(1000u32, &1u64);
        let bal2 = pallet_assets::Pallet::<Test>::balance(1000u32, &2u64);
        assert_eq!(bal1, 750u128);
        assert_eq!(bal2, 250u128);

        // Unauthorized mint by non-root should fail
        assert_noop!(
            Tcc::mint(frame_system::RawOrigin::Signed(1u64).into(), 1u64.into(), 100u128),
            pallet_tcc::Error::<Test>::NotAuthorized
        );

        // Burn 50 from account 2 by Root
        assert_ok!(Tcc::burn(frame_system::RawOrigin::Root.into(), 2u64.into(), 50u128));
        let bal2_after = pallet_assets::Pallet::<Test>::balance(1000u32, &2u64);
        assert_eq!(bal2_after, 200u128);
    });
}
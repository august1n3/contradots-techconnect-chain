//! Mock runtime for pallet-tcc tests (wraps pallet-assets).

#![cfg(test)]

use sp_core::H256;
use sp_runtime::{testing::Header, traits::{BlakeTwo256, IdentityLookup}};
use frame_support::{parameter_types, construct_runtime};
use frame_support::traits::Everything;

use crate as pallet_tcc;
use pallet_assets as assets;

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type AssetId = u32;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const ExistentialDeposit: Balance = 1;
    pub const AssetDeposit: u128 = 0;
    pub const AssetAccountDeposit: u128 = 0;
    pub const MetadataDepositBase: u128 = 0;
    pub const MetadataDepositPerByte: u128 = 0;
    pub const ApprovalDeposit: u128 = 0;
    pub const StringLimit: u32 = 50;
}

construct_runtime!(
    pub enum Test where
        Block = frame_system::mocking::MockBlock<Test>,
        NodeBlock = frame_system::mocking::MockBlock<Test>,
        UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: assets::{Pallet, Call, Storage, Event<T>},
        Tcc: pallet_tcc::{Pallet, Call, Storage, Event<T>},
    }
);

// system
impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type Header = Header;
    type RuntimeEvent = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

// balances
impl pallet_balances::Config for Test {
    type RuntimeEvent = Event;
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = ();
    type WeightInfo = ();
}

// assets
impl assets::Config for Test {
    type RuntimeEvent = Event;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
}

// tcc config
parameter_types! {
    pub const TccAssetIdParam: u32 = 1000u32;
}
impl pallet_tcc::Config for Test {
    type RuntimeEvent = Event;
    type Currency = Assets;
    type AssetId = AssetId;
    type Balance = Balance;
    type TccAssetId = frame_support::traits::ConstU32<{ TccAssetIdParam }>;
    type InstantiateOrigin = frame_system::EnsureRoot<AccountId>;
    type MintOrigin = frame_system::EnsureRoot<AccountId>;
    type BurnOrigin = frame_system::EnsureRoot<AccountId>;
    type WeightInfo = ();
}

// helper: build test ext
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    // seed balances (used for fees & asset-backed transfers)
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1u64, 1_000_000u128), (2u64, 1_000_000u128), (99u64, 1_000_000u128)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
//! Mock runtime for pallet-member-registry tests.

#![cfg(test)]

use sp_core::H256;
use sp_runtime::{testing::Header, traits::{BlakeTwo256, IdentityLookup}};
use frame_support::{parameter_types, construct_runtime};
use frame_support::traits::Everything;

use crate as pallet_member_registry;

pub type AccountId = u64;
pub type BlockNumber = u64;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const MinimumPeriod: u64 = 1;
}

construct_runtime!(
    pub enum Test where
        Block = frame_system::mocking::MockBlock<Test>,
        NodeBlock = frame_system::mocking::MockBlock<Test>,
        UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        MemberRegistry: pallet_member_registry::{Pallet, Call, Storage, Event<T>},
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
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

// timestamp
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

// balances
parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}
impl pallet_balances::Config for Test {
    type RuntimeEvent = Event;
    type Balance = u128;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = ();
    type WeightInfo = ();
}

// member-registry config
impl pallet_member_registry::Config for Test {
    type RuntimeEvent = Event;
    type RootClubAdminOrigin = frame_system::EnsureRoot<AccountId>;
    type Time = Timestamp;
    type WeightInfo = ();
}

// helper: build test ext
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1u64, 1_000_000u128), (2u64, 1_000_000u128), (3u64, 1_000_000u128)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
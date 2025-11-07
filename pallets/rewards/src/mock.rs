//! Mock runtime for pallet-rewards tests.


#![cfg(test)]

use sp_core::H256;
use sp_runtime::{testing::Header, traits::{BlakeTwo256, IdentityLookup}};
use frame_support::{parameter_types, construct_runtime};
use frame_support::traits::Everything;

use crate as pallet_rewards;
use pallet_member_registry as member_registry;

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const ExistentialDeposit: Balance = 1;
    pub const EpochLengthInBlocks: u64 = 10;
    pub const MaxAttestations: u32 = 1_000_000;
    pub const MaxAttestationsPerSubject: u32 = 1024;
    pub const MaxMetadataLenReward: u32 = 256;
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
        MemberRegistry: member_registry::{Pallet, Call, Storage, Event<T>},
        Rewards: pallet_rewards::{Pallet, Call, Storage, Event<T>},
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

// timestamp
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
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

// member-registry wiring
impl member_registry::Config for Test {
    type RuntimeEvent = Event;
    type RootClubAdminOrigin = frame_system::EnsureRoot<AccountId>;
    type Time = Timestamp;
    type WeightInfo = ();
}

// rewards config
impl pallet_rewards::Config for Test {
    type RuntimeEvent = Event;
    type Currency = Balances;
    type RuleCreationOrigin = frame_system::EnsureRoot<AccountId>;
    type ManualAwardOrigin = frame_system::EnsureRoot<AccountId>;
    type TimeProvider = Timestamp;
    type EpochLengthInBlocks = frame_support::traits::ConstU64<EpochLengthInBlocks>;
    type MaxMetadataLen = frame_support::traits::ConstU32<MaxMetadataLenReward>;
    type MaxAttestations = frame_support::traits::ConstU32<MaxAttestations>;
    type MaxAttestationsPerSubject = frame_support::traits::ConstU32<MaxAttestationsPerSubject>;
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
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    construct_runtime,
    parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Everything},
    weights::constants::RocksDbWeight,
};
use sp_core::H256;
use sp_runtime::{
    generic,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32,
};
use sp_version::RuntimeVersion;

// Basic primitive types
pub type AccountId = AccountId32;
pub type Balance = u128;
pub type BlockNumber = u32;
pub type Index = u32;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Signature = sp_runtime::MultiSignature;

pub struct DummyWeight;

// Parameter types
parameter_types! {
    pub const BlockHashCount: u32 = 2400;
    pub const SS58Prefix: u8 = 42;
    pub const ExistentialDeposit: Balance = 1;
    pub const MinimumPeriod: u64 = 3;
    pub const ParachainId: u32 = 2000; 
}

pub const PARACHAIN_ID: u32 = ParachainId::get();

// Construct runtime FIRST
construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Sudo: pallet_sudo,
        MemberRegistry: pallet_member_registry,
    }
);

// System config
impl frame_system::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;

    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;

    // Additional associated types required by recent FRAME versions
    type RuntimeTask = ();
    type Nonce = Index;
    type Block = generic::Block<Header, UncheckedExtrinsic>;
    type ExtensionsWeightInfo = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();

    // Common
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = RocksDbWeight;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type BlockHashCount = BlockHashCount;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

// Implement dummy weights for Member Registry pallet
impl pallet_member_registry::WeightInfo for DummyWeight {
    fn create_club() -> frame_support::weights::Weight { frame_support::weights::Weight::from_parts(0, 0) }
    fn add_officer() -> frame_support::weights::Weight { frame_support::weights::Weight::from_parts(0, 0) }
    fn remove_officer() -> frame_support::weights::Weight { frame_support::weights::Weight::from_parts(0, 0) }
    fn create_attestation() -> frame_support::weights::Weight { frame_support::weights::Weight::from_parts(0, 0) }
    fn register_member() -> frame_support::weights::Weight { frame_support::weights::Weight::from_parts(0, 0) }
    fn add_member_admin() -> frame_support::weights::Weight { frame_support::weights::Weight::from_parts(0, 0) }
    fn remove_member_admin() -> frame_support::weights::Weight { frame_support::weights::Weight::from_parts(0, 0) }
    fn set_role() -> frame_support::weights::Weight { frame_support::weights::Weight::from_parts(0, 0) }
    // Include any other functions defined in your pallet::WeightInfo
}

// Timestamp
impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<3u64>;
    type WeightInfo = ();
}

// Balances
impl pallet_balances::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type DustRemoval = ();
    // Use the parameter type directly
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = ();
    type WeightInfo = ();

    // Recent associated types
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type ReserveIdentifier = ();
    type FreezeIdentifier = ();
    type MaxReserves = ();
    type MaxFreezes = ();
    type DoneSlashHandler = ();
}

// Sudo
impl pallet_sudo::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

// Member Registry
impl pallet_member_registry::Config for Runtime {
    type RootClubAdminOrigin = frame_system::EnsureRoot<AccountId>;
    type Time = pallet_timestamp::Pallet<Runtime>;
    type WeightInfo = DummyWeight;
}

// Extrinsic types (after Runtime exists)
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));


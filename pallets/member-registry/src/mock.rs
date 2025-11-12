//! Mock runtime for pallet-member-registry tests.
//! Place this file as `pallets/member-registry/src/mock.rs` and `mod mock;` from your tests.

#![cfg(test)]

use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
};
use sp_runtime::BuildStorage;
// use pallet_balances::Event;
// use frame_system::Call;
// use frame_system::Origin;
use frame_support::{
    parameter_types,
    construct_runtime,
    traits::{Everything,},
};
use crate as pallet_member_registry;

// --- Type aliases used in the mock runtime ---
pub type AccountId = u64;
pub type Balance = u128;
// pub type BlockNumber = u64;
pub type Nonce = u64;
pub struct DummyWeight;
// pub type DevAccountTuple = (u32, u128, Option<String>);

// --- Parameter types ---
parameter_types! {
    pub const BlockHashCount: u32 = 2400;
    pub const SS58Prefix: u8 = 42;
    pub const MinimumPeriod: u64 = 1;
    pub const ExistentialDeposit: Balance = 1;
}

// --- Construct a minimal Test runtime ---
// IMPORTANT: Newer Polkadot SDK / FRAME versions no longer use a `where` clause with Block;
// we set the Block associated type directly in frame_system::Config.
construct_runtime!(
    pub enum Test {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        MemberRegistry: pallet_member_registry,
    }
);

// --- frame_system::Config (updated for newer FRAME with additional associated types) ---
impl frame_system::Config for Test {
    // Core associated types
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;


    // Newer required associated types (provide simple defaults)
    type RuntimeTask = ();               // if tasks framework unused
    type Nonce = Nonce;
    type Block = frame_system::mocking::MockBlock<Test>;
    type ExtensionsWeightInfo = ();      // no extension weights in tests
    type SingleBlockMigrations = ();     // none for mock
    type MultiBlockMigrator = ();        // none for mock
    type PreInherents = ();              // no custom callbacks
    type PostInherents = ();
    type PostTransactions = ();

    // Legacy / common items
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type BlockHashCount = BlockHashCount;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

// --- Timestamp ---
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

// --- Balances ---
// NOTE: Newer versions may require extra associated types (hold/freeze reasons). Provide unit defaults.
impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = ();
    type WeightInfo = ();

    // Additional associated types in recent versions:
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type ReserveIdentifier = ();
    type FreezeIdentifier = ();
    type MaxReserves = ();
    type MaxFreezes = ();
    type DoneSlashHandler = ();
}

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

// --- Member Registry ---
impl pallet_member_registry::Config for Test {
   type WeightInfo = DummyWeight;
   
   #[doc = " Origin that can perform privileged cluster-wide membership ops (e.g., initial club creation)."]
    #[doc = " In many deployments this can be `EnsureRoot<Self::RuntimeOrigin>`."]
    type RootClubAdminOrigin = frame_system::EnsureRoot<Self::RuntimeOrigin>;
   
   #[doc = " Time provider - used for timestamping attestations and joins."]
    type Time = pallet_timestamp::Pallet<Self>;

}

// --- TestExternalities builder ---
pub fn new_test_ext() -> sp_io::TestExternalities {
    // IMPORTANT: include the generic argument `<Test>` on GenesisConfig
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("frame_system storage");

    // Initialize pallet_timestamp storage
    

    // Newer pallet_balances::GenesisConfig may include an extra field (e.g., dev_accounts).
    // If your version demands it, supply an empty vec; if not needed, remove dev_accounts line.
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 1_000_000),
            (2, 1_000_000),
            (3, 1_000_000),
            (10, 1_000_000), // club admin
            (99, 1_000_000),
        ],
        dev_accounts: Some((10, 100, None)),
    }
    .build_storage()
    .expect("balances storage")
    .top.into_iter()
    .for_each(|(k, v)| {storage.top.insert(k, v);});

    let mut ext = sp_io::TestExternalities::new(storage);

    ext.execute_with(|| {
        // This initializes the timestamp storage (sets it to 0).
        <pallet_timestamp::Pallet<Test> as frame_support::traits::OnGenesis>::on_genesis(); 
        
        // Then, manually set the block number and time for tests.
        System::set_block_number(1);
        Timestamp::set_timestamp(1); // Set the block time to 1
    });

    ext.execute_with(|| System::set_block_number(1));
    ext
}
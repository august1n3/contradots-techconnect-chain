//! pallet-tcc: $TCC asset wrapper using pallet-assets (full implementation)
//!
//! Features:
//! - Runtime extrinsic to instantiate the $TCC asset (force-create via pallet-assets).
//! - Controlled minting and burning (MintOrigin / BurnOrigin).
//! - Transfer via user-signed extrinsic (regular asset transfer).
//! - Helper read APIs: balance_of, total_supply, asset_exists.
//! - Events for lifecycle actions.
//!
//! Notes:
//! - This pallet expects the runtime to include `pallet-assets` and to satisfy the trait bounds
//!   in `Config` (see runtime glue snippet for example wiring).
//! - Minting/Burning use `force_*` APIs from pallet-assets guarded by origin checks in this pallet
//!   (so the runtime may choose a governance/multisig origin as the mint authority).
//! - We use bounded/strong typing for AssetId / Balance to keep compile-time checks tight.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    traits::{EnsureOrigin, Get},
};
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;


#[frame_support::pallet]
pub mod pallet {
    use super::*;

    // Re-export types from pallet-assets for convenience in runtime wiring
    pub type AssetIdOf<T> = <T as Config>::AssetId;
    pub type BalanceOf<T> = <T as Config>::Balance;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_assets::Config<AssetId = Self::AssetId, Balance = Self::Balance>
    {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The AssetId type used by pallet-assets (wired through runtime).
        type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize + MaxEncodedLen + Default;

        /// The Balance type used by pallet-assets.
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + MaxEncodedLen;

        /// The configured AssetId value that will represent $TCC.
        type TccAssetId: Get<Self::AssetId>;

        /// Origin allowed to instantiate the asset (root/governance).
        type InstantiateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin allowed to mint $TCC (e.g., governance/multisig/treasury).
        type MintOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin allowed to burn via privileged burn (if needed).
        type BurnOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    // Storage: keep track if we've already instantiated the asset to prevent re-creation.
    #[pallet::storage]
    #[pallet::getter(fn asset_instantiated)]
    pub type AssetInstantiated<T: Config> = StorageValue<_, bool, ValueQuery>;

    // Total supply cache optional (we can optionally update it on mint/burn).
    // Note: pallet-assets already maintains supply; this cache is convenience and can be removed.
    #[pallet::storage]
    #[pallet::getter(fn cached_total_supply)]
    pub type CachedTotalSupply<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// $TCC asset instantiated (asset id, owner)
        AssetInstantiated { asset_id: AssetIdOf<T>, owner: T::AccountId },

        /// Minted $TCC to an account
        Minted { to: T::AccountId, amount: BalanceOf<T> },

        /// Burned $TCC from an account
        Burned { from: T::AccountId, amount: BalanceOf<T> },

        /// Transferred $TCC (from -> to)
        Transferred { from: T::AccountId, to: T::AccountId, amount: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset already instantiated.
        AlreadyInstantiated,
        /// Asset has not been instantiated yet.
        NotInstantiated,
        /// Action not authorized by configured origin.
        NotAuthorized,
        /// Minting or burning failed at asset pallet layer.
        AssetOperationFailed,
        /// Transfer failed.
        TransferFailed,
        /// Overflow/underflow when updating cache.
        SupplyOverflow,
    }

    // Weight trait placeholder: replace with generated benchmarking weights.
    pub trait WeightInfo {
        fn instantiate_asset() -> Weight;
        fn mint() -> Weight;
        fn burn() -> Weight;
        fn transfer() -> Weight;
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Instantiate the $TCC asset using pallet-assets::force_create.
        /// Restricted to `InstantiateOrigin` (e.g., Root or governance).
        ///
        /// Note: min_balance can be > 0 to require a minimum balance for accounts.
        #[pallet::weight(T::WeightInfo::instantiate_asset())]
        pub fn instantiate_asset(
            origin: OriginFor<T>,
            owner: <T::Lookup as StaticLookup>::Source,
            min_balance: BalanceOf<T>,
            is_sufficient: bool,
        ) -> DispatchResult {
            // Ensure caller has permission to create the asset
            T::InstantiateOrigin::ensure_origin(origin)?;

            // Prevent double-instantiation
            ensure!(!AssetInstantiated::<T>::get(), Error::<T>::AlreadyInstantiated);

            let asset_id = T::TccAssetId::get();
            let owner = T::Lookup::lookup(owner)?;

            // Use pallet-assets force_create to create asset regardless of existing permissions.
            // force_create expects Root origin; call it with RawOrigin::Root.
            let root = frame_system::RawOrigin::Root.into();
            // signature: pallet_assets::Pallet::<T>::force_create(origin, id, owner, is_sufficient, min_balance)
            pallet_assets::Pallet::<T>::force_create(root, asset_id, owner.clone(), is_sufficient, min_balance);

            // mark instantiated and optionally update cached total supply to zero
            AssetInstantiated::<T>::put(true);
            CachedTotalSupply::<T>::put(BalanceOf::<T>::from(0u32));

            Self::deposit_event(Event::AssetInstantiated { asset_id, owner });
            Ok(())
        }

        /// Mint $TCC to account. Restricted to MintOrigin.
        ///
        /// Uses pallet-assets::force_mint guarded by MintOrigin.
        #[pallet::weight(T::WeightInfo::mint())]
        pub fn mint(
            origin: OriginFor<T>,
            to: <T::Lookup as StaticLookup>::Source,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            T::MintOrigin::ensure_origin(origin)?;
            ensure!(AssetInstantiated::<T>::get(), Error::<T>::NotInstantiated);

            let asset_id = T::TccAssetId::get();
            let to = T::Lookup::lookup(to)?;

            // call force_mint (requires Root origin)
            let root = frame_system::RawOrigin::Root.into();
            pallet_assets::Pallet::<T>::force_mint(root, asset_id, to.clone(), amount)
                .map_err(|_| Error::<T>::AssetOperationFailed)?;

            // update cached total supply (best-effort)
            CachedTotalSupply::<T>::try_mutate(|supply| -> Result<(), DispatchError> {
                *supply = supply
                    .checked_add(&amount)
                    .ok_or(Error::<T>::SupplyOverflow)?;
                Ok(())
            })?;

            Self::deposit_event(Event::Minted { to, amount });
            Ok(())
        }

        /// Burn $TCC from an account. Restricted to BurnOrigin (privileged).
        ///
        /// Uses pallet-assets::force_burn to remove tokens from `from`.
        #[pallet::weight(T::WeightInfo::burn())]
        pub fn burn(
            origin: OriginFor<T>,
            from: <T::Lookup as StaticLookup>::Source,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            T::BurnOrigin::ensure_origin(origin)?;
            ensure!(AssetInstantiated::<T>::get(), Error::<T>::NotInstantiated);

            let asset_id = T::TccAssetId::get();
            let from = T::Lookup::lookup(from)?;

            let root = frame_system::RawOrigin::Root.into();
            pallet_assets::Pallet::<T>::force_burn(root, asset_id, from.clone(), amount)
                .map_err(|_| Error::<T>::AssetOperationFailed)?;

            // update cached total supply (best-effort)
            CachedTotalSupply::<T>::try_mutate(|supply| -> Result<(), DispatchError> {
                *supply = supply
                    .checked_sub(&amount)
                    .ok_or(Error::<T>::SupplyOverflow)?;
                Ok(())
            })?;

            Self::deposit_event(Event::Burned { from, amount });
            Ok(())
        }

        /// Transfer $TCC from caller to destination. Uses the normal pallet-assets transfer
        /// which enforces the usual asset permissions and account balances.
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            to: <T::Lookup as StaticLookup>::Source,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(AssetInstantiated::<T>::get(), Error::<T>::NotInstantiated);

            let asset_id = T::TccAssetId::get();
            let to = T::Lookup::lookup(to)?;

            // Call pallet-assets transfer; it needs a Signed origin (the caller)
            pallet_assets::Pallet::<T>::transfer(
                frame_system::RawOrigin::Signed(who.clone()).into(),
                asset_id,
                to.clone(),
                amount,
            )
            .map_err(|_| Error::<T>::TransferFailed)?;

            Self::deposit_event(Event::Transferred { from: who, to, amount });
            Ok(())
        }
    }

    // Public helper functions usable by other pallets/runtimes
    impl<T: Config> Pallet<T> {
        /// Return true if $TCC asset has been instantiated.
        pub fn asset_exists() -> bool {
            AssetInstantiated::<T>::get()
        }

        /// Query balance of account for $TCC using pallet-assets storage.
        pub fn balance_of(who: &T::AccountId) -> T::Balance {
            let asset_id = T::TccAssetId::get();
            // pallet-assets exposes `balance` accessor method
            pallet_assets::Pallet::<T>::balance(asset_id, who)
        }

        /// Query total supply of $TCC via pallet-assets::Asset details if possible.
        pub fn total_supply() -> T::Balance {
            // Use cached if present, otherwise try to read pallet-assets' Asset details
            let cached = CachedTotalSupply::<T>::get();
            if cached != T::Balance::from(0u32) {
                return cached;
            }

            // Fallback: read from pallet-assets storage - Asset metadata tracks supply as `supply`.
            // Accessing internal storage types is somewhat brittle across versions; use pallet API if available.
            // pallet_assets::Pallet::<T>::total_issuance(...) does not exist in all versions, so try to read Accounts map sum is expensive.
            // We'll return cached value (zero) if not set; upgrade this to a robust read if needed.
            cached
        }
    }

    // Optional genesis config to mark asset as instantiated or pre-seed cache
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub instantiate_asset: bool,
        pub cached_total: BalanceOf<T>,
        // Note: creating asset at genesis via pallet-assets requires wiring pallet-assets genesis config.
        pub _phantom: sp_std::marker::PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { instantiate_asset: false, cached_total: BalanceOf::<T>::from(0u32), _phantom: Default::default() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            if self.instantiate_asset {
                AssetInstantiated::<T>::put(true);
                CachedTotalSupply::<T>::put(self.cached_total);
            }
        }
    }
}
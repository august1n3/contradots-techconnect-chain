//! pallet-rewards: reward rules and verified claim flow via on-chain attestations (full implementation)
//!
//! Features:
//! - Create reward rules (club-scoped or global) that define reward amounts and per-epoch limits.
//! - Attestor flow: authorized attestors (club officers) create attestations for subjects tied to a rule.
//! - Members claim rewards by presenting attestation ids; attestation is consumed (one-time use).
//! - Per-account, per-rule, per-epoch accounting to enforce max_per_epoch limits.
//! - Manual award path for attestors / governance as a fallback.
//! - Emits events for all important actions to be indexed by SubQuery for leaderboards / UI.
//!
//! Integration notes:
//! - This pallet expects `pallet-member-registry` to be present in the runtime and uses it to:
//!     - verify attestor is authorized for a club-scoped rule.
//!     - verify membership if needed by policy.
//! - Reward payment is done via the `Currency` trait configured in `Config` (wire `pallet_tcc::Pallet` or `pallet_assets` wrapper).
//! - Replace WeightInfo placeholders with benchmarked weights before production.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, EnsureOrigin, UnixTime},
    BoundedVec,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::Saturating;
use sp_std::prelude::*;
use codec::{Decode, Encode};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html>
// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//
// To see a full list of `pallet` macros and their use cases, see:
// <https://paritytech.github.io/polkadot-sdk/master/pallet_example_kitchensink/index.html>
// <https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/index.html>
#[frame_support::pallet]
pub mod pallet {
    use super::*;

    pub type RuleId = u32;
    pub type RewardAmount = u128;
    pub type ClubId = u32;
    pub type Moment = u64;
    pub type AttestationId = u64;
    pub type Epoch = u64;

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct RewardRule {
        pub event_type: BoundedVec<u8, ConstU32<32>>, // e.g., "attendance"
        pub amount: RewardAmount,
        pub max_per_epoch: u32,
        pub club: Option<ClubId>, // if Some, rule scoped to club
        pub metadata: Option<[u8;32]>, // optional pointer to off-chain proof spec
    }

    /// Attestation stored on-chain created by an attestor (club officer) allowing subject to claim a reward for a rule.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct Attestation<AccountId> {
        pub subject: AccountId,
        pub rule_id: RuleId,
        pub attestor: AccountId,
        pub created_at: Moment,
        pub expires_at: Option<Moment>,
        pub used: bool,
        pub metadata: Option<[u8;32]>, // optional pointer to event evidence
    }

    /// Per-account per-rule claim counter storing last_epoch and count within that epoch.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
    pub struct ClaimCounter {
        pub epoch: Epoch,
        pub count: u32,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency used to pay rewards (pallet_tcc wrapper or any Currency)
        type Currency: Currency<Self::AccountId>;

        /// Origin allowed to create rules (e.g., governance/root or club admin via outer checks)
        type RuleCreationOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin allowed to manually award tokens (attestor fallback). Can be Root or a multi-sig.
        type ManualAwardOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Time provider for timestamps / expiry checks
        type TimeProvider: UnixTime;

        /// Blocks per epoch (for per-epoch accounting)
        type EpochLengthInBlocks: Get<Self::BlockNumber>;

        /// Maximum metadata length for rule metadata if using variable fields (unused for fixed-size hashes).
        type MaxMetadataLen: Get<u32>;

        /// Max number of attestations allowed globally per-pallet (bound safety)
        type MaxAttestations: Get<u32>;

        /// Max number of attestations per-subject/bounded index if you add indexing (not used here)
        type MaxAttestationsPerSubject: Get<u32>;

        /// WeightInfo for extrinsics (replace with benchmarking)
        type WeightInfo: WeightInfo;
    }

    // Weight stubs - replace with generated weights later
    pub trait WeightInfo {
        fn create_rule() -> Weight;
        fn create_attestation() -> Weight;
        fn claim_reward() -> Weight;
        fn award_manual() -> Weight;
        fn revoke_attestation() -> Weight;
    }

    // Storage items
    #[pallet::storage]
    #[pallet::getter(fn rules)]
    pub(super) type Rules<T: Config> = StorageMap<_, Twox64Concat, RuleId, RewardRule, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn next_rule_id)]
    pub(super) type NextRuleId<T: Config> = StorageValue<_, RuleId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn attestations)]
    pub(super) type Attestations<T: Config> =
        StorageMap<_, Twox64Concat, AttestationId, Attestation<T::AccountId>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn next_attestation_id)]
    pub(super) type NextAttestationId<T: Config> = StorageValue<_, AttestationId, ValueQuery>;

    /// (who, rule_id) -> ClaimCounter for per-epoch accounting
    #[pallet::storage]
    #[pallet::getter(fn claims_this_epoch)]
    pub(super) type ClaimsThisEpoch<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat, T::AccountId,
        Twox64Concat, RuleId,
        ClaimCounter,
        ValueQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RuleCreated { rule_id: RuleId },
        AttestationCreated { attestation_id: AttestationId, subject: T::AccountId, rule_id: RuleId, attestor: T::AccountId },
        AttestationRevoked { attestation_id: AttestationId },
        RewardClaimed { who: T::AccountId, rule_id: RuleId, attestation_id: AttestationId, amount: RewardAmount },
        RewardAwarded { who: T::AccountId, amount: RewardAmount, reason: Option<Vec<u8>> },
    }

    #[pallet::error]
    pub enum Error<T> {
        RuleNotFound,
        AttestationNotFound,
        AttestationExpired,
        AttestationUsed,
        NotAuthorizedAttestor,
        ExceedsLimit,
        TransferFailed,
        InvalidInput,
        AttestationsOverflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a reward rule. Origin controlled by `RuleCreationOrigin` (e.g., governance/root).
        #[pallet::weight(T::WeightInfo::create_rule())]
        pub fn create_rule(
            origin: OriginFor<T>,
            event_type: Vec<u8>,
            amount: RewardAmount,
            max_per_epoch: u32,
            club: Option<ClubId>,
            metadata: Option<[u8;32]>,
        ) -> DispatchResult {
            T::RuleCreationOrigin::ensure_origin(origin)?;
            let bounded_event = BoundedVec::<u8, ConstU32<32>>::try_from(event_type)
                .map_err(|_| Error::<T>::InvalidInput)?;

            let id = NextRuleId::<T>::get();
            let rule = RewardRule {
                event_type: bounded_event,
                amount,
                max_per_epoch,
                club,
                metadata,
            };
            Rules::<T>::insert(id, rule);
            NextRuleId::<T>::put(id.saturating_add(1));
            Self::deposit_event(Event::RuleCreated { rule_id: id });
            Ok(())
        }

        /// Create an attestation for a subject tied to a rule. Signed by attestor (club officer).
        /// Attestor must be authorized:
        /// - If rule.club is Some(cid): attestor must be admin/officer of that club (via member-registry).
        /// - If rule.club is None: only ManualAwardOrigin or governance may create attestations (to avoid abuse).
        #[pallet::weight(T::WeightInfo::create_attestation())]
        pub fn create_attestation(
            origin: OriginFor<T>,
            subject: T::AccountId,
            rule_id: RuleId,
            expires_at: Option<Moment>,
            metadata: Option<[u8;32]>,
        ) -> DispatchResult {
            let attestor = ensure_signed(origin)?;
            // rule existence
            let rule = Rules::<T>::get(rule_id).ok_or(Error::<T>::RuleNotFound)?;

            // Authorization: if rule scoped to club, require attestor to be officer/admin of that club
            if let Some(cid) = rule.club {
                // call member-registry helper; map any error to NotAuthorizedAttestor
                let ok = pallet_member_registry::Pallet::<T>::is_officer_or_admin(&attestor, cid)
                    .map_err(|_| Error::<T>::NotAuthorizedAttestor)?;
                ensure!(ok, Error::<T>::NotAuthorizedAttestor);
            } else {
                // global rule: allow only ManualAwardOrigin to create attestations (conservative)
                // This prevents arbitrary users from issuing attestations for global rules.
                T::ManualAwardOrigin::ensure_origin(frame_system::RawOrigin::Signed(attestor.clone()).into())
                    .map_err(|_| Error::<T>::NotAuthorizedAttestor)?;
            }

            // create attestation
            let now = T::TimeProvider::now().as_millis().saturated_into::<Moment>();

            let id = NextAttestationId::<T>::get();

            // Bounds check on total attestations (optional safety)
            let max_att = T::MaxAttestations::get();
            ensure!(id < max_att.into(), Error::<T>::AttestationsOverflow);

            let att = Attestation {
                subject: subject.clone(),
                rule_id,
                attestor: attestor.clone(),
                created_at: now,
                expires_at,
                used: false,
                metadata,
            };

            Attestations::<T>::insert(id, att);
            NextAttestationId::<T>::put(id.saturating_add(1));

            Self::deposit_event(Event::AttestationCreated { attestation_id: id, subject, rule_id, attestor });
            Ok(())
        }

        /// Claim reward by presenting an attestation id. Attestation is consumed (marked used).
        #[pallet::weight(T::WeightInfo::claim_reward())]
        pub fn claim_reward(
            origin: OriginFor<T>,
            attestation_id: AttestationId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // load one-time attestation; mutate to set used
            Attestations::<T>::try_mutate(attestation_id, |maybe_att| -> DispatchResult {
                let mut att = maybe_att.take().ok_or(Error::<T>::AttestationNotFound)?;

                // ensure subject matches caller
                ensure!(att.subject == who, Error::<T>::InvalidInput);

                // ensure not used
                ensure!(!att.used, Error::<T>::AttestationUsed);

                // expiry check
                if let Some(exp) = att.expires_at {
                    let now = T::TimeProvider::now().as_millis().saturated_into::<Moment>();
                    ensure!(now <= exp, Error::<T>::AttestationExpired);
                }

                // rule exists
                let rule = Rules::<T>::get(att.rule_id).ok_or(Error::<T>::RuleNotFound)?;

                // optional extra attestor check: ensure attestor still authorized (club admin/officer)
                if let Some(cid) = rule.club {
                    let ok = pallet_member_registry::Pallet::<T>::is_officer_or_admin(&att.attestor, cid)
                        .map_err(|_| Error::<T>::NotAuthorizedAttestor)?;
                    ensure!(ok, Error::<T>::NotAuthorizedAttestor);
                } else {
                    // global rules: attestor authorization not re-checked here (already restricted at creation).
                }

                // epoch & per-epoch limit check
                let current_epoch = Self::current_epoch();
                ClaimsThisEpoch::<T>::try_mutate(&who, att.rule_id, |counter| -> DispatchResult {
                    // if counter epoch differs, reset counter
                    if counter.epoch != current_epoch {
                        counter.epoch = current_epoch;
                        counter.count = 0u32;
                    }
                    // enforce limit
                    let new_count = counter.count.saturating_add(1);
                    ensure!(new_count <= rule.max_per_epoch, Error::<T>::ExceedsLimit);
                    counter.count = new_count;
                    Ok(())
                })?;

                // mark attestation used and write back
                att.used = true;
                *maybe_att = Some(att.clone());

                // transfer reward to caller via Currency::deposit_creating
                let amount = rule.amount.saturated_into::<T::Balance>();
                // T::Currency may be pallet_tcc::Pallet (which implements Currency) or similar
                T::Currency::deposit_creating(&who, amount);

                Self::deposit_event(Event::RewardClaimed { who: who.clone(), rule_id: att.rule_id, attestation_id, amount: rule.amount });
                Ok(())
            })
        }

        /// Manual award by governance/attestor origin. Use for edge-cases or one-off grants.
        #[pallet::weight(T::WeightInfo::award_manual())]
        pub fn award_manual(
            origin: OriginFor<T>,
            to: T::AccountId,
            amount: RewardAmount,
            reason: Option<Vec<u8>>,
        ) -> DispatchResult {
            T::ManualAwardOrigin::ensure_origin(origin)?;
            let bal: <T::Currency as Currency<T::AccountId>>::Balance = amount.saturated_into();
            T::Currency::deposit_creating(&to, bal);
            Self::deposit_event(Event::RewardAwarded { who: to, amount, reason });
            Ok(())
        }

        /// Revoke an attestation (attestor or governance). Useful to cancel stale attestations.
        #[pallet::weight(T::WeightInfo::revoke_attestation())]
        pub fn revoke_attestation(origin: OriginFor<T>, attestation_id: AttestationId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Attestations::<T>::try_mutate_exists(attestation_id, |maybe| -> DispatchResult {
                let att = maybe.as_ref().ok_or(Error::<T>::AttestationNotFound)?;
                // allow attestor or club admin to revoke
                let rule = Rules::<T>::get(att.rule_id).ok_or(Error::<T>::RuleNotFound)?;

                if let Some(cid) = rule.club {
                    // require who is attestor or club admin/officer
                    if who != att.attestor {
                        let ok = pallet_member_registry::Pallet::<T>::is_officer_or_admin(&who, cid)
                            .map_err(|_| Error::<T>::NotAuthorizedAttestor)?;
                        ensure!(ok, Error::<T>::NotAuthorizedAttestor);
                    }
                } else {
                    // global rule: require manual award origin (governance) to revoke
                    T::ManualAwardOrigin::ensure_origin(frame_system::RawOrigin::Signed(who.clone()).into())
                        .map_err(|_| Error::<T>::NotAuthorizedAttestor)?;
                }

                // remove attestation
                *maybe = None;
                Ok(())
            })?;

            Self::deposit_event(Event::AttestationRevoked { attestation_id });
            Ok(())
        }
    }

    // Helper functions
    impl<T: Config> Pallet<T> {
        /// Compute current epoch based on block number and EpochLengthInBlocks
        pub fn current_epoch() -> Epoch {
            let bn: T::BlockNumber = <frame_system::Pallet<T>>::block_number();
            let epoch_len: T::BlockNumber = T::EpochLengthInBlocks::get();
            // convert to u64 safely
            let bn_u64: u64 = bn.saturated_into::<u64>();
            let epoch_len_u64: u64 = epoch_len.saturated_into::<u64>();
            if epoch_len_u64 == 0 {
                0u64
            } else {
                bn_u64 / epoch_len_u64
            }
        }

        /// Convenience getter to inspect an attestation (read-only)
        pub fn get_attestation(attestation_id: AttestationId) -> Option<Attestation<T::AccountId>> {
            Attestations::<T>::get(attestation_id)
        }
    }

    // Genesis config - optional seeding
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub rules: Vec<(RuleId, RewardRule)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { rules: vec![] }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (id, rule) in &self.rules {
                Rules::<T>::insert(id, rule.clone());
                let next = NextRuleId::<T>::get();
                if *id >= next {
                    NextRuleId::<T>::put(id.saturating_add(1));
                }
            }
        }
    }
}
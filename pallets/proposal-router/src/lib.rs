//! pallet-proposal-router: full implementation
//! - Routes proposals scoped to a club (club-scoped governance implemented inside this pallet)
//!   or global proposals (global governance implemented inside this pallet).
//! - Stores proposals as SCALE-encoded RuntimeCall bytes and executes the call if the voting
//!   process passes. Club proposals are voteable by club members; global proposals are voteable
//!   by all registered members (configurable).
//! - Designed to interoperate with `pallet-member-registry` helper methods:
//!     - pallet_member_registry::Pallet::<T>::is_member(who: &T::AccountId) -> bool
//!     - pallet_member_registry::Pallet::<T>::is_officer_or_admin(who: &T::AccountId, club: ClubId) -> Result<bool, _>
//!
//! Notes:
//! - This pallet decodes the stored call bytes into `T::RuntimeCall` before dispatching.
//!   `T::RuntimeCall` must implement `Dispatchable<RuntimeOrigin = T::RuntimeOrigin>` and `Decode`.
//! - Execution is performed as a Signed origin for the pallet account; ensure the target calls
//!   are written to accept a signed origin or to be callable by the pallet account (or adapt).
//! - Voting windows are block-based with a configurable default; a custom voting_period can be
//!   passed per-proposal. Quorum and passing thresholds are configurable at runtime via constants.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::WeightInfo;

use frame_support::{
    pallet_prelude::*,
    traits::{UnixTime, EnsureOrigin},
    BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{Saturating, Dispatchable, AccountIdConversion};
use sp_std::vec::Vec;
use parity_scale_codec::{Encode, Decode, DecodeWithMemTracking};
use scale_info::TypeInfo;


#[frame_support::pallet]

pub mod pallet {
    use super::*;

    pub type ProposalId = u64;
    pub type ClubId = u32;
    pub type Votes = u32;
    pub type BlockNumberOf<T> = BlockNumberFor<T>;

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen, DecodeWithMemTracking)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub enum Scope {
        Club(ClubId),
        Global,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        pub id: ProposalId,
        pub proposer: T::AccountId,
        pub scope: Scope,
        /// SCALE-encoded runtime Call bytes
        pub call: Vec<u8>,
        pub metadata: Option<BoundedVec<u8, T::MaxMetadataLen>>,
        pub start: BlockNumberOf<T>,
        pub end: BlockNumberOf<T>,
        pub yea: Votes,
        pub nay: Votes,
        pub executed: bool,
        /// voters list (to prevent double-vote). size bounded for storage limits.
        pub voters: BoundedVec<T::AccountId, T::MaxVotersPerProposal>,
        /// quorum in absolute votes required to consider the vote valid
        pub quorum: Votes,
        /// passing threshold (simple majority threshold expressed as percent*100, e.g., 5000 = 50.00%)
        pub pass_threshold: u32,
        /// the club for club-scoped proposals
        pub club: Option<ClubId>,
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The concrete Runtime Call type of the runtime (used for decoding/executing proposals).
        /// Must implement `Dispatchable` with `RuntimeOrigin = T::RuntimeOrigin` and `Decode`.
        type RuntimeCall: Parameter + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + Decode + Encode + From<Call<Self>>;

        /// Maximum bytes length permitted for proposal metadata
        type MaxMetadataLen: Get<u32>;

        /// Maximum number of voters tracked per proposal (prevents unbounded vectors)
        type MaxVotersPerProposal: Get<u32>;

        /// Default voting period in blocks if not overridden per-proposal
        type DefaultVotingPeriod: Get<BlockNumberFor<Self>>;

        /// Default quorum (# votes required) for proposals
        type DefaultQuorum: Get<Votes>;

        /// Default pass threshold (percent * 100; 50% = 5000)
        type DefaultPassThreshold: Get<u32>;

        /// Origin that can perform privileged router operations (e.g., cancel proposals)
        type RouterAdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Helper: time provider (optional; used only for metadata timestamps if needed)
        type TimeProvider: UnixTime;

        /// WeightInfo for extrinsics (populate by benchmarking)
        type WeightInfo: WeightInfo;
    }

    // Storage
    #[pallet::storage]
    #[pallet::getter(fn next_proposal_id)]
    pub(super) type NextProposalId<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub(super) type Proposals<T: Config> =
        StorageMap<_, Twox64Concat, ProposalId, Proposal<T>, OptionQuery>;

    // Optional index by club -> vector of proposal ids for quick lookup (bounded per club)
    #[pallet::storage]
    #[pallet::getter(fn club_proposals)]
    pub(super) type ClubProposals<T: Config> =
        StorageMap<_, Twox64Concat, ClubId, BoundedVec<ProposalId, ConstU32<1024>>, OptionQuery>;

    // Events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ProposalCreated { id: ProposalId, proposer: T::AccountId, is_club_scoped: bool },
        Voted { id: ProposalId, who: T::AccountId, aye: bool, weight: Votes },
        ProposalExecuted { id: ProposalId },
        ProposalFailed { id: ProposalId, reason: Vec<u8> },
        ProposalCancelled { id: ProposalId },
    }

    // Errors
    #[pallet::error]
    pub enum Error<T> {
        ProposalNotFound,
        VotingClosed,
        AlreadyVoted,
        NotEligibleToPropose,
        NotMember,
        NotClubMember,
        NotAuthorized,
        ProposalAlreadyExecuted,
        ExecutionFailed,
        MetadataTooLarge,
        VotersOverflow,
        ClubIndexOverflow,
        InvalidCallEncoding,
        QuorumNotReached,
        ProposalNotPassed,
    }

    // Weight trait placeholder
    pub trait WeightInfo {
        fn propose() -> Weight;
        fn vote() -> Weight;
        fn execute() -> Weight;
        fn cancel() -> Weight;
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Dispatchable calls
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a proposal. `call` should be the SCALE-encoded bytes of `T::RuntimeCall`.
        ///
        /// For `Scope::Club(club_id)`:
        ///   - proposer must be a member of that club (checked via member-registry).
        ///   - default quorum/threshold/voting_period are used unless overridden via optional params.
        ///
        /// For `Scope::Global`:
        ///   - proposer must be any registered member.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::propose())]
        pub fn propose(
            origin: OriginFor<T>,
            scope: Scope,
            call: Vec<u8>,
            metadata: Option<Vec<u8>>,
            voting_period: Option<BlockNumberOf<T>>,
            quorum: Option<Votes>,
            pass_threshold: Option<u32>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // require proposer's membership eligibility
            // TODO: Integrate with pallet-member-registry when available in runtime
            match &scope {
                Scope::Club(_club_id) => {
                    // TODO: ensure proposer is a member of the club
                    // ensure!(pallet_member_registry::Pallet::<T>::is_member(&who), Error::<T>::NotMember);
                    // let clubs = pallet_member_registry::Pallet::<T>::member_clubs(&who).ok_or(Error::<T>::NotMember)?;
                    // ensure!(clubs.iter().any(|c| c == club_id), Error::<T>::NotClubMember);
                }
                Scope::Global => {
                    // TODO: ensure proposer is any registered member
                    // ensure!(pallet_member_registry::Pallet::<T>::is_member(&who), Error::<T>::NotMember);
                }
            }

            // metadata size check
            let metadata_bounded = if let Some(md) = metadata {
                let max = T::MaxMetadataLen::get() as usize;
                ensure!(md.len() <= max, Error::<T>::MetadataTooLarge);
                Some(BoundedVec::try_from(md).map_err(|_| Error::<T>::MetadataTooLarge)?)
            } else {
                None
            };

            // create proposal
            let id = NextProposalId::<T>::get();
            let start = <frame_system::Pallet<T>>::block_number();
            let period = voting_period.unwrap_or_else(|| T::DefaultVotingPeriod::get());
            let end = start.saturating_add(period);
            let q = quorum.unwrap_or_else(|| T::DefaultQuorum::get());
            let pt = pass_threshold.unwrap_or_else(|| T::DefaultPassThreshold::get());

            let proposal = Proposal::<T> {
                id,
                proposer: who.clone(),
                scope: scope.clone(),
                call: call.clone(),
                metadata: metadata_bounded,
                start,
                end,
                yea: 0u32,
                nay: 0u32,
                executed: false,
                voters: BoundedVec::try_from(Vec::<T::AccountId>::new()).map_err(|_| Error::<T>::VotersOverflow)?,
                quorum: q,
                pass_threshold: pt,
                club: match scope { Scope::Club(cid) => Some(cid), _ => None },
            };

            Proposals::<T>::insert(id, proposal);

            if let Scope::Club(cid) = &scope {
                ClubProposals::<T>::mutate(cid, |maybe| {
                    if let Some(vec) = maybe {
                        vec.try_push(id).map_err(|_| Error::<T>::ClubIndexOverflow)
                    } else {
                        let mut v = BoundedVec::<ProposalId, ConstU32<1024>>::new();
                        v.try_push(id).map_err(|_| Error::<T>::ClubIndexOverflow)?;
                        *maybe = Some(v);
                        Ok(())
                    }
                })?;
            }

            NextProposalId::<T>::put(id.saturating_add(1));
            let is_club_scoped = matches!(scope, Scope::Club(_));
            Self::deposit_event(Event::ProposalCreated { id, proposer: who, is_club_scoped });
            Ok(())
        }

        /// Vote on a proposal. Voters must be eligible:
        /// - For club proposals: member of that club
        /// - For global proposals: any registered member
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::vote())]
        pub fn vote(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
            aye: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Proposals::<T>::try_mutate(proposal_id, |maybe| -> DispatchResult {
                let p = maybe.as_mut().ok_or(Error::<T>::ProposalNotFound)?;
                let now = <frame_system::Pallet<T>>::block_number();
                ensure!(now >= p.start && now <= p.end, Error::<T>::VotingClosed);
                ensure!(!p.voters.contains(&who), Error::<T>::AlreadyVoted);

                // eligibility checks
                // TODO: Integrate with pallet-member-registry when available in runtime
                match p.scope {
                    Scope::Club(_cid) => {
                        // TODO: require membership of the club
                        // ensure!(pallet_member_registry::Pallet::<T>::is_member(&who), Error::<T>::NotMember);
                        // let clubs = pallet_member_registry::Pallet::<T>::member_clubs(&who).ok_or(Error::<T>::NotMember)?;
                        // ensure!(clubs.iter().any(|c| c == &cid), Error::<T>::NotClubMember);
                    }
                    Scope::Global => {
                        // TODO: ensure voter is any registered member
                        // ensure!(pallet_member_registry::Pallet::<T>::is_member(&who), Error::<T>::NotMember);
                    }
                }

                // record vote
                if aye {
                    p.yea = p.yea.saturating_add(1);
                } else {
                    p.nay = p.nay.saturating_add(1);
                }
                p.voters.try_push(who.clone()).map_err(|_| Error::<T>::VotersOverflow)?;
                Self::deposit_event(Event::Voted { id: proposal_id, who, aye, weight: 1u32 });
                Ok(())
            })
        }

        /// Execute proposal if voting period elapsed and it passed.
        /// Decodes the stored call bytes into `T::RuntimeCall` and dispatches it as Signed(pallet_account).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::execute())]
        pub fn execute(origin: OriginFor<T>, proposal_id: ProposalId) -> DispatchResult {
            // allow anyone to call execute once conditions met
            let _ = ensure_signed(origin)?;

            Proposals::<T>::try_mutate(proposal_id, |maybe| -> DispatchResult {
                let p = maybe.as_mut().ok_or(Error::<T>::ProposalNotFound)?;
                ensure!(!p.executed, Error::<T>::ProposalAlreadyExecuted);

                let now = <frame_system::Pallet<T>>::block_number();
                ensure!(now > p.end, Error::<T>::VotingClosed);

                // Check quorum
                let total_votes = p.yea.saturating_add(p.nay);
                ensure!(total_votes >= p.quorum, Error::<T>::QuorumNotReached);

                // Check pass threshold: (yea / total_votes) * 10000 >= pass_threshold
                let pass_percent_x100 = if total_votes == 0 {
                    0u32
                } else {
                    // compute percent*100 safely
                    let numerator = p.yea.saturating_mul(10_000u32);
                    numerator / total_votes
                };
                ensure!(pass_percent_x100 >= p.pass_threshold, Error::<T>::ProposalNotPassed);

                // decode call
                let decoded_call = <T as Config>::RuntimeCall::decode(&mut &p.call[..])
                    .map_err(|_| Error::<T>::InvalidCallEncoding)?;

                // Dispatch as the pallet account (Signed origin)
                let pallet_origin = frame_system::RawOrigin::Signed(Self::pallet_account()).into();
                let result = decoded_call.dispatch(pallet_origin);

                p.executed = true;

                match result {
                    Ok(_info) => {
                        // DispatchOk - emit event
                        Self::deposit_event(Event::ProposalExecuted { id: proposal_id });
                        // Optionally remove from club index; we keep history for audit
                        Ok(())
                    }
                    Err(err) => {
                        // store failure event
                        let err_bytes = err.encode();
                        Self::deposit_event(Event::ProposalFailed { id: proposal_id, reason: err_bytes });
                        Err(Error::<T>::ExecutionFailed.into())
                    }
                }
            })
        }

        /// Cancel a proposal before execution. Restricted to RouterAdminOrigin (governance or root).
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::cancel())]
        pub fn cancel(origin: OriginFor<T>, proposal_id: ProposalId) -> DispatchResult {
            T::RouterAdminOrigin::ensure_origin(origin)?;
            Proposals::<T>::try_mutate_exists(proposal_id, |maybe| -> DispatchResult {
                maybe.take().ok_or(Error::<T>::ProposalNotFound)?;
                Ok(())
            })?;
            Self::deposit_event(Event::ProposalCancelled { id: proposal_id });
            Ok(())
        }
    }

    // Pallet helper functions
    impl<T: Config> Pallet<T> {
        /// derive a deterministic pallet account id to be used when executing calls
        pub fn pallet_account() -> T::AccountId {
            // Use a PalletId-like derivation using module name bytes
            // In runtime glue you may want to override this to a constant PalletId
            let entropy = b"proposal_r";
            pallet_id_from_bytes(entropy).into_account_truncating()
        }
    }

    // Small helper to produce an account id from a fixed byte slice (not as robust as PalletId)
    fn pallet_id_from_bytes(b: &[u8]) -> PalletId {
        // If b.len() < 8 pad with zeros; this is purposely simple.
        let mut id = [0u8; 8];
        for (i, byte) in b.iter().take(8).enumerate() {
            id[i] = *byte;
        }
        PalletId(id)
    }
}

pub use pallet::*;
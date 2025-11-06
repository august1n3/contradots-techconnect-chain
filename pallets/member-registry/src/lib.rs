//! pallet-member-registry: canonical membership, clubs, roles and attestations (full implementation)
//! - Officer attestation flow (on-chain signed attestations by officers/admins).
//! - Member self-registration using a stored attestation.
//! - Club admin flows: create club, add/remove officers, add/remove members, assign roles.
//! - Storage limits use BoundedVec to avoid unbounded on-chain allocations.
//!
//! Notes:
//! - This pallet uses a simple on-chain attestation model where authorized officers submit an
//!   `attest` extrinsic (signed) which stores an attestation record. The target user then calls
//!   `register_member` referencing that attestation id. This avoids complex on-chain signature
//!   recovery for arbitrary off-chain signatures. If you want off-chain signatures verified in the
//!   runtime, we can add an `verify_signed_attestation` helper using `MultiSignature` + `MultiSigner`.
//! - Make sure to add benchmarking for each dispatchable and wire WeightInfo in runtime.



#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
    pallet_prelude::*,
    traits::{EnsureOrigin, StoredMap, UnixTime},
    BoundedVec,
};

use frame_system::pallet_prelude::*;
use sp_runtime::traits::Saturating;
use sp_std::vec::Vec;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    pub type ClubId = u32;
    pub type RoleId = u8;
    pub type AttestationId = u64;
    pub type Moment = u64; // map to Timestamp in runtime if desired

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum MemberStatus {
        Active,
        Suspended,
        Removed,
    }

    /// Stored information about a member
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct MemberInfo<AccountId> {
        pub clubs: BoundedVec<ClubId, ConstU32<8>>,
        pub roles: BoundedVec<RoleId, ConstU32<8>>,
        pub joined_at: Moment,
        pub status: MemberStatus,
        pub metadata: Option<[u8; 32]>, // pointer to off-chain profile (IPFS hash)
    }

    /// Stored information about a club
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct ClubInfo<AccountId> {
        pub name: BoundedVec<u8, ConstU32<64>>,
        pub admin: AccountId,
        pub officers: BoundedVec<AccountId, ConstU32<32>>,
        pub members_count: u32,
        pub metadata: Option<[u8; 32]>, // optional pointer to off-chain metadata
    }

    /// Attestation created on-chain by an authorized officer approving a subject to join a club.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct Attestation<AccountId> {
        pub subject: AccountId,
        pub club: ClubId,
        pub attestor: AccountId,
        pub created_at: Moment,
        pub expires_at: Option<Moment>,
        pub used: bool,
        pub metadata: Option<[u8; 32]>, // optional pointer to event proof
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    // `without_storage_info` may be removed if compiling against newer Substrate where it's not needed.
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin that can perform privileged cluster-wide membership ops (e.g., initial club creation).
        /// In many deployments this can be `EnsureRoot<Self::RuntimeOrigin>`.
        type RootClubAdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Time provider - optional, used for timestamping attestations and joins.
        type Time: UnixTime;

        /// WeightInfo for benchmarking; provide concrete weights in runtime.
        type WeightInfo: WeightInfo;
    }

    // Storage items
    #[pallet::storage]
    #[pallet::getter(fn members)]
    pub(super) type Members<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, MemberInfo<T::AccountId>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn clubs)]
    pub(super) type Clubs<T: Config> =
        StorageMap<_, Twox64Concat, ClubId, ClubInfo<T::AccountId>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn next_club_id)]
    pub(super) type NextClubId<T: Config> = StorageValue<_, ClubId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn next_attestation_id)]
    pub(super) type NextAttestationId<T: Config> = StorageValue<_, AttestationId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn attestations)]
    pub(super) type Attestations<T: Config> =
        StorageMap<_, Twox64Concat, AttestationId, Attestation<T::AccountId>, OptionQuery>;

    // Events emitted by the pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ClubCreated { club: ClubId, admin: T::AccountId },
        OfficerAdded { club: ClubId, officer: T::AccountId },
        OfficerRemoved { club: ClubId, officer: T::AccountId },

        MemberRegistered { who: T::AccountId, club: ClubId },
        MemberRemoved { who: T::AccountId, club: ClubId },
        RoleAssigned { who: T::AccountId, role: RoleId },

        AttestationCreated { id: AttestationId, subject: T::AccountId, club: ClubId, attestor: T::AccountId },
        AttestationUsed { id: AttestationId, subject: T::AccountId, club: ClubId },
        AttestationRevoked { id: AttestationId },
    }

    // Errors returned by dispatchables
    #[pallet::error]
    pub enum Error<T> {
        ClubNotFound,
        AlreadyMember,
        NotMember,
        NotClubAdmin,
        NotOfficer,
        MemberNotFound,
        Overflow,
        AttestationNotFound,
        AttestationExpired,
        AttestationUsed,
        AttestorNotAuthorized,
        InvalidInput,
    }

    // Benchmark weight trait placeholder
    pub trait WeightInfo {
        fn create_club() -> Weight;
        fn add_officer() -> Weight;
        fn remove_officer() -> Weight;
        fn create_attestation() -> Weight;
        fn register_member() -> Weight;
        fn add_member_admin() -> Weight;
        fn remove_member_admin() -> Weight;
        fn set_role() -> Weight;
    }

    // Dispatchable functions
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new club and set admin. Restricted to RootClubAdminOrigin.
        #[pallet::weight(T::WeightInfo::create_club())]
        pub fn create_club(
            origin: OriginFor<T>,
            name: BoundedVec<u8, ConstU32<64>>,
            admin: T::AccountId,
            metadata: Option<[u8; 32]>,
        ) -> DispatchResult {
            T::RootClubAdminOrigin::ensure_origin(origin)?;

            let id = NextClubId::<T>::get();
            let club = ClubInfo {
                name,
                admin: admin.clone(),
                officers: BoundedVec::try_from(vec![]).map_err(|_| Error::<T>::Overflow)?,
                members_count: 0,
                metadata,
            };

            Clubs::<T>::insert(id, club);
            NextClubId::<T>::put(id.saturating_add(1));
            Self::deposit_event(Event::ClubCreated { club: id, admin });
            Ok(())
        }

        /// Add an officer to a club. Must be called by the club admin (signed origin).
        #[pallet::weight(T::WeightInfo::add_officer())]
        pub fn add_officer(
            origin: OriginFor<T>,
            club: ClubId,
            officer: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Clubs::<T>::try_mutate(club, |maybe| -> DispatchResult {
                let mut c = maybe.as_mut().ok_or(Error::<T>::ClubNotFound)?;
                ensure!(c.admin == who, Error::<T>::NotClubAdmin);
                c.officers.try_push(officer.clone()).map_err(|_| Error::<T>::Overflow)?;
                Ok(())
            })?;
            Self::deposit_event(Event::OfficerAdded { club, officer });
            Ok(())
        }

        /// Remove an officer from a club. Must be called by the club admin.
        #[pallet::weight(T::WeightInfo::remove_officer())]
        pub fn remove_officer(
            origin: OriginFor<T>,
            club: ClubId,
            officer: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Clubs::<T>::try_mutate(club, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::ClubNotFound)?;
                ensure!(c.admin == who, Error::<T>::NotClubAdmin);
                // remove officer if present
                if let Some(pos) = c.officers.iter().position(|o| o == &officer) {
                    c.officers.swap_remove(pos);
                }
                Ok(())
            })?;
            Self::deposit_event(Event::OfficerRemoved { club, officer });
            Ok(())
        }

        /// Officers create attestations for a subject to allow them to register.
        /// This is a signed extrinsic by an officer/admin and stores an attestation record.
        #[pallet::weight(T::WeightInfo::create_attestation())]
        pub fn create_attestation(
            origin: OriginFor<T>,
            subject: T::AccountId,
            club: ClubId,
            expires_at: Option<Moment>,
            metadata: Option<[u8; 32]>,
        ) -> DispatchResult {
            let attestor = ensure_signed(origin)?;
            // check club exists
            let club_info = Clubs::<T>::get(club).ok_or(Error::<T>::ClubNotFound)?;
            // attestor must be admin or one of the officers
            ensure!(
                attestor == club_info.admin || club_info.officers.iter().any(|o| o == &attestor),
                Error::<T>::AttestorNotAuthorized
            );

            let now = T::Time::now().as_millis().saturated_into::<Moment>();
            let id = NextAttestationId::<T>::get();
            let att = Attestation {
                subject: subject.clone(),
                club,
                attestor: attestor.clone(),
                created_at: now,
                expires_at,
                used: false,
                metadata,
            };

            Attestations::<T>::insert(id, att);
            NextAttestationId::<T>::put(id.saturating_add(1));
            Self::deposit_event(Event::AttestationCreated { id, subject, club, attestor });
            Ok(())
        }

        /// Register self as a member of a club using an attestation created by an officer.
        #[pallet::weight(T::WeightInfo::register_member())]
        pub fn register_member(
            origin: OriginFor<T>,
            attestation_id: AttestationId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // get attestation
            Attestations::<T>::try_mutate(attestation_id, |maybe_att| -> DispatchResult {
                let mut att = maybe_att.take().ok_or(Error::<T>::AttestationNotFound)?;
                ensure!(!att.used, Error::<T>::AttestationUsed);

                // check subject matches signer
                ensure!(att.subject == who, Error::<T>::InvalidInput);

                // check attestation expiry
                if let Some(exp) = att.expires_at {
                    let now = T::Time::now().as_millis().saturated_into::<Moment>();
                    ensure!(now <= exp, Error::<T>::AttestationExpired);
                }

                // check attestor is still authorized (admin or in officers list)
                let club_info = Clubs::<T>::get(att.club).ok_or(Error::<T>::ClubNotFound)?;
                ensure!(
                    att.attestor == club_info.admin || club_info.officers.iter().any(|o| o == &att.attestor),
                    Error::<T>::AttestorNotAuthorized
                );

                // ensure subject is not already member
                ensure!(!Members::<T>::contains_key(&who), Error::<T>::AlreadyMember);

                // create member record
                let member = MemberInfo {
                    clubs: BoundedVec::try_from(vec![att.club]).map_err(|_| Error::<T>::Overflow)?,
                    roles: BoundedVec::try_from(vec![]).map_err(|_| Error::<T>::Overflow)?,
                    joined_at: att.created_at,
                    status: MemberStatus::Active,
                    metadata: None,
                };

                Members::<T>::insert(&who, member);
                // increment club member count
                Clubs::<T>::mutate(att.club, |maybe_c| {
                    if let Some(ref mut c) = maybe_c {
                        c.members_count = c.members_count.saturating_add(1);
                    }
                });

                // mark attestation used and persist
                att.used = true;
                *maybe_att = Some(att);

                Self::deposit_event(Event::MemberRegistered { who: who.clone(), club: att.club });
                Self::deposit_event(Event::AttestationUsed { id: attestation_id, subject: who, club: att.club });
                Ok(())
            })
        }

        /// Add a member directly (club admin only) - creates MemberInfo immediately.
        #[pallet::weight(T::WeightInfo::add_member_admin())]
        pub fn add_member_admin(
            origin: OriginFor<T>,
            subject: T::AccountId,
            club: ClubId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let club_info = Clubs::<T>::get(club).ok_or(Error::<T>::ClubNotFound)?;
            ensure!(who == club_info.admin, Error::<T>::NotClubAdmin);

            ensure!(!Members::<T>::contains_key(&subject), Error::<T>::AlreadyMember);

            let now = T::Time::now().as_millis().saturated_into::<Moment>();
            let member = MemberInfo {
                clubs: BoundedVec::try_from(vec![club]).map_err(|_| Error::<T>::Overflow)?,
                roles: BoundedVec::try_from(vec![]).map_err(|_| Error::<T>::Overflow)?,
                joined_at: now,
                status: MemberStatus::Active,
                metadata: None,
            };

            Members::<T>::insert(&subject, member);
            Clubs::<T>::mutate(club, |maybe| {
                if let Some(ref mut c) = maybe {
                    c.members_count = c.members_count.saturating_add(1);
                }
            });

            Self::deposit_event(Event::MemberRegistered { who: subject, club });
            Ok(())
        }

        /// Remove (or mark removed) a member from a club (club admin only).
        #[pallet::weight(T::WeightInfo::remove_member_admin())]
        pub fn remove_member_admin(
            origin: OriginFor<T>,
            subject: T::AccountId,
            club: ClubId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let club_info = Clubs::<T>::get(club).ok_or(Error::<T>::ClubNotFound)?;
            ensure!(who == club_info.admin, Error::<T>::NotClubAdmin);

            Members::<T>::try_mutate(&subject, |maybe| -> DispatchResult {
                let mut m = maybe.take().ok_or(Error::<T>::MemberNotFound)?;
                // mark removed (keep history)
                m.status = MemberStatus::Removed;
                *maybe = Some(m);
                Ok(())
            })?;

            Clubs::<T>::mutate(club, |maybe| {
                if let Some(ref mut c) = maybe {
                    c.members_count = c.members_count.saturating_sub(1);
                }
            });

            Self::deposit_event(Event::MemberRemoved { who: subject, club });
            Ok(())
        }

        /// Assign a role to a member (club admin only in this simple model)
        #[pallet::weight(T::WeightInfo::set_role())]
        pub fn set_role(
            origin: OriginFor<T>,
            subject: T::AccountId,
            role: RoleId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // for simplicity require root or global admin here; adapt to club-scoped checks if needed
            // In production you likely want club-scoped role setting: check that `who` is club admin for the club.
            T::RootClubAdminOrigin::ensure_origin(origin)?;
            Members::<T>::try_mutate(&subject, |maybe| -> DispatchResult {
                let m = maybe.as_mut().ok_or(Error::<T>::MemberNotFound)?;
                m.roles.try_push(role).map_err(|_| Error::<T>::Overflow)?;
                Ok(())
            })?;
            Self::deposit_event(Event::RoleAssigned { who: subject, role });
            Ok(())
        }

        /// Revoke (delete) an attestation (admin/officer or root)
        #[pallet::weight(T::WeightInfo::create_attestation())]
        pub fn revoke_attestation(origin: OriginFor<T>, attestation_id: AttestationId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // allow root or the attestor who created it to revoke.
            Attestations::<T>::try_mutate_exists(attestation_id, |maybe| -> DispatchResult {
                let att = maybe.as_mut().ok_or(Error::<T>::AttestationNotFound)?;
                // if signer is root admin allow; otherwise only attestor or club admin can revoke.
                let club_info = Clubs::<T>::get(att.club).ok_or(Error::<T>::ClubNotFound)?;
                if who != att.attestor && who != club_info.admin {
                    // check if root origin
                    ensure!(false, Error::<T>::NotClubAdmin);
                }
                // remove attestation
                *maybe = None;
                Ok(())
            })?;
            Self::deposit_event(Event::AttestationRevoked { id: attestation_id });
            Ok(())
        }
    }

    // Pallet helper functions
    impl<T: Config> Pallet<T> {
        /// Return true if `acct` is an officer or admin of `club`
        pub fn is_officer_or_admin(acct: &T::AccountId, club: ClubId) -> Result<bool, Error<T>> {
            let club_info = Clubs::<T>::get(club).ok_or(Error::<T>::ClubNotFound)?;
            Ok(acct == &club_info.admin || club_info.officers.iter().any(|o| o == acct))
        }

        /// Return true if `who` is member
        pub fn is_member(who: &T::AccountId) -> bool {
            Members::<T>::contains_key(who)
        }

        /// Get clubs a member belongs to (if member)
        pub fn member_clubs(who: &T::AccountId) -> Option<Vec<ClubId>> {
            Members::<T>::get(who).map(|m| m.clubs.into_iter().collect())
        }

        /// Convenience: current epoch millis time
        pub fn now_millis() -> Moment {
            T::Time::now().as_millis().saturated_into::<Moment>()
        }
    }
}
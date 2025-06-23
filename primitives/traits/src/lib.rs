// Copyright (C) Moondance Labs Ltd.
// This file is part of Tanssi.

// Tanssi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Tanssi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Tanssi.  If not, see <http://www.gnu.org/licenses/>

//! Crate containing various traits used by moondance crates allowing to connect pallet
//! with each other or with mocks.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod alias;
pub mod prod_or_fast;

pub use {
    alias::*,
    cumulus_primitives_core::{
        relay_chain::{BlockNumber, HeadData, Slot, ValidationCode},
        ParaId,
    },
    dp_chain_state_snapshot::{GenericStateProof, ReadEntryErr},
    dp_container_chain_genesis_data::ContainerChainGenesisDataItem,
};
use {
    core::marker::PhantomData,
    frame_support::{
        dispatch::DispatchErrorWithPostInfo,
        pallet_prelude::{Decode, DispatchResultWithPostInfo, Encode, Get, MaxEncodedLen, Weight},
        BoundedVec,
    },
    scale_info::TypeInfo,
    serde::{Deserialize, Serialize},
    sp_core::H256,
    sp_runtime::{
        app_crypto::sp_core,
        traits::{CheckedAdd, CheckedMul},
        ArithmeticError, DispatchResult, Perbill, RuntimeDebug,
    },
    sp_std::{
        collections::{btree_map::BTreeMap, btree_set::BTreeSet},
        vec::Vec,
    },
};

// Separate import as rustfmt wrongly change it to `sp_std::vec::self`, which is the module instead
// of the macro.
use sp_std::vec;

/// The collator-assignment hook to react to collators being assigned to container chains.
pub trait CollatorAssignmentHook<Balance> {
    /// This hook is called when collators are assigned to a container
    ///
    /// The hook should never panic and is required to return the weight consumed.
    fn on_collators_assigned(
        para_id: ParaId,
        maybe_tip: Option<&Balance>,
        is_parathread: bool,
    ) -> Result<Weight, sp_runtime::DispatchError>;
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<Balance> CollatorAssignmentHook<Balance> for Tuple {
    fn on_collators_assigned(
        p: ParaId,
        t: Option<&Balance>,
        ip: bool,
    ) -> Result<Weight, sp_runtime::DispatchError> {
        let mut weight: Weight = Default::default();
        for_tuples!( #( weight.saturating_accrue(Tuple::on_collators_assigned(p, t, ip)?); )* );
        Ok(weight)
    }
}

/// Container chains collator assignment tip prioritization on congestion.
/// Tips paras are willing to pay for collator assignment in case of collators demand
/// surpasses the offer.
pub trait CollatorAssignmentTip<Balance> {
    fn get_para_tip(a: ParaId) -> Option<Balance>;
}

impl<Balance> CollatorAssignmentTip<Balance> for () {
    fn get_para_tip(_: ParaId) -> Option<Balance> {
        None
    }
}

pub struct AuthorNotingInfo<AccountId> {
    pub author: AccountId,
    pub block_number: BlockNumber,
    pub para_id: ParaId,
}

/// The author-noting hook to react to container chains authoring.
pub trait AuthorNotingHook<AccountId> {
    /// This hook is called partway through the `set_latest_author_data` inherent in author-noting.
    ///
    /// The hook should never panic and is required to return the weight consumed.
    fn on_container_authors_noted(info: &[AuthorNotingInfo<AccountId>]) -> Weight;

    #[cfg(feature = "runtime-benchmarks")]
    fn prepare_worst_case_for_bench(author: &AccountId, block_number: BlockNumber, para_id: ParaId);
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<AccountId> AuthorNotingHook<AccountId> for Tuple {
    fn on_container_authors_noted(info: &[AuthorNotingInfo<AccountId>]) -> Weight {
        let mut weight: Weight = Default::default();
        for_tuples!( #( weight.saturating_accrue(Tuple::on_container_authors_noted(info)); )* );
        weight
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn prepare_worst_case_for_bench(a: &AccountId, b: BlockNumber, p: ParaId) {
        for_tuples!( #( Tuple::prepare_worst_case_for_bench(a, b, p); )* );
    }
}

pub trait DistributeRewards<AccountId, Imbalance> {
    fn distribute_rewards(rewarded: AccountId, amount: Imbalance) -> DispatchResultWithPostInfo;
}

impl<AccountId, Imbalance> DistributeRewards<AccountId, Imbalance> for () {
    fn distribute_rewards(_rewarded: AccountId, _amount: Imbalance) -> DispatchResultWithPostInfo {
        Ok(().into())
    }
}

/// Get the current list of container chains parachain ids.
pub trait GetCurrentContainerChains {
    type MaxContainerChains: Get<u32>;

    fn current_container_chains() -> BoundedVec<ParaId, Self::MaxContainerChains>;

    #[cfg(feature = "runtime-benchmarks")]
    fn set_current_container_chains(container_chains: &[ParaId]);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ForSession {
    Current,
    Next,
}

/// Get the current list of container chains parachain ids with its assigned collators.
/// It can return a para id with an empty list of collators.
pub trait GetContainerChainsWithCollators<AccountId> {
    fn container_chains_with_collators(for_session: ForSession) -> Vec<(ParaId, Vec<AccountId>)>;

    fn get_all_collators_assigned_to_chains(for_session: ForSession) -> BTreeSet<AccountId>;

    #[cfg(feature = "runtime-benchmarks")]
    fn set_container_chains_with_collators(
        for_session: ForSession,
        container_chains: &[(ParaId, Vec<AccountId>)],
    );
}

/// How often should a parathread collator propose blocks. The units are "1 out of n slots", where the slot time is the
/// tanssi slot time, 6 seconds.
// TODO: this is currently ignored
#[derive(
    Clone,
    Debug,
    Encode,
    Decode,
    scale_info::TypeInfo,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    MaxEncodedLen,
)]
pub struct SlotFrequency {
    /// The parathread will produce at most 1 block every x slots. min=10 means that collators can produce 1 block
    /// every `x >= 10` slots, but they are not enforced to. If collators produce a block after less than 10
    /// slots, they will not be rewarded by tanssi.
    pub min: u32,
    /// The parathread will produce at least 1 block every x slots. max=10 means that collators are forced to
    /// produce 1 block every `x <= 10` slots. Collators can produce a block sooner than that if the `min` allows it, but
    /// waiting more than 10 slots will make them lose the block reward.
    pub max: u32,
}

impl SlotFrequency {
    pub fn should_parathread_buy_core(
        &self,
        current_slot: Slot,
        max_slot_required_to_complete_purchase: Slot,
        last_block_slot: Slot,
    ) -> bool {
        current_slot
            >= last_block_slot
                .saturating_add(Slot::from(u64::from(self.min)))
                .saturating_sub(max_slot_required_to_complete_purchase)
    }

    pub fn should_parathread_author_block(
        &self,
        current_slot: Slot,
        last_block_slot: Slot,
    ) -> bool {
        current_slot >= last_block_slot.saturating_add(Slot::from(u64::from(self.min)))
    }
}

impl Default for SlotFrequency {
    fn default() -> Self {
        Self { min: 1, max: 1 }
    }
}

#[derive(
    Clone,
    Debug,
    Encode,
    Decode,
    scale_info::TypeInfo,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    MaxEncodedLen,
)]
pub struct ParathreadParams {
    pub slot_frequency: SlotFrequency,
}

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct SessionContainerChains {
    pub parachains: Vec<ParaId>,
    pub parathreads: Vec<(ParaId, ParathreadParams)>,
}

/// Get the list of container chains parachain ids at given
/// session index.
pub trait GetSessionContainerChains<SessionIndex> {
    fn session_container_chains(session_index: SessionIndex) -> SessionContainerChains;
    #[cfg(feature = "runtime-benchmarks")]
    fn set_session_container_chains(session_index: SessionIndex, container_chains: &[ParaId]);
}

/// Returns author for a parachain id for the given slot.
pub trait GetContainerChainAuthor<AccountId> {
    fn author_for_slot(slot: Slot, para_id: ParaId) -> Option<AccountId>;
    #[cfg(feature = "runtime-benchmarks")]
    fn set_authors_for_para_id(para_id: ParaId, authors: Vec<AccountId>);
}

/// Returns the host configuration composed of the amount of collators assigned
/// to the orchestrator chain, and how many collators are assigned per container chain.
pub trait GetHostConfiguration<SessionIndex> {
    fn max_collators(session_index: SessionIndex) -> u32;
    fn min_collators_for_orchestrator(session_index: SessionIndex) -> u32;
    fn max_collators_for_orchestrator(session_index: SessionIndex) -> u32;
    fn collators_per_container(session_index: SessionIndex) -> u32;
    fn collators_per_parathread(session_index: SessionIndex) -> u32;
    fn target_container_chain_fullness(session_index: SessionIndex) -> Perbill;
    fn max_parachain_cores_percentage(session_index: SessionIndex) -> Option<Perbill>;
    fn full_rotation_mode(session_index: SessionIndex) -> FullRotationModes;
    #[cfg(feature = "runtime-benchmarks")]
    fn set_host_configuration(_session_index: SessionIndex) {}
}

/// Returns current session index.
pub trait GetSessionIndex<SessionIndex> {
    fn session_index() -> SessionIndex;

    #[cfg(feature = "runtime-benchmarks")]
    fn skip_to_session(session_index: SessionIndex);
}

/// Should pallet_collator_assignment trigger a full rotation on this session?
pub trait ShouldRotateAllCollators<SessionIndex> {
    fn should_rotate_all_collators(session_index: SessionIndex) -> bool;
}

impl<SessionIndex> ShouldRotateAllCollators<SessionIndex> for () {
    fn should_rotate_all_collators(_session_index: SessionIndex) -> bool {
        false
    }
}

/// Helper trait for pallet_collator_assignment to be able to give priority to invulnerables
pub trait RemoveInvulnerables<AccountId> {
    /// Remove the first n invulnerables from the list of collators. The order should be respected.
    fn remove_invulnerables(
        collators: &mut Vec<AccountId>,
        num_invulnerables: usize,
    ) -> Vec<AccountId>;
}

impl<AccountId: Clone> RemoveInvulnerables<AccountId> for () {
    fn remove_invulnerables(
        _collators: &mut Vec<AccountId>,
        _num_invulnerables: usize,
    ) -> Vec<AccountId> {
        // Default impl: no collators are invulnerables
        vec![]
    }
}

/// Helper trait for pallet_collator_assignment to be able to not assign collators to container chains with no credits
/// in pallet_services_payment
pub trait ParaIdAssignmentHooks<B, AC> {
    /// Remove para ids with not enough credits. The resulting order will affect priority: the first para id in the list
    /// will be the first one to get collators.
    fn pre_assignment(para_ids: &mut Vec<ParaId>, old_assigned: &BTreeSet<ParaId>);
    fn post_assignment(
        current_assigned: &BTreeSet<ParaId>,
        new_assigned: &mut BTreeMap<ParaId, Vec<AC>>,
        maybe_tip: &Option<B>,
    ) -> Weight;

    /// Make those para ids valid by giving them enough credits, for benchmarking.
    #[cfg(feature = "runtime-benchmarks")]
    fn make_valid_para_ids(para_ids: &[ParaId]);
}

impl<B, AC> ParaIdAssignmentHooks<B, AC> for () {
    fn pre_assignment(_para_ids: &mut Vec<ParaId>, _currently_assigned: &BTreeSet<ParaId>) {}

    fn post_assignment(
        _current_assigned: &BTreeSet<ParaId>,
        _new_assigned: &mut BTreeMap<ParaId, Vec<AC>>,
        _maybe_tip: &Option<B>,
    ) -> Weight {
        Default::default()
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn make_valid_para_ids(_para_ids: &[ParaId]) {}
}

pub trait RelayStorageRootProvider {
    fn get_relay_storage_root(relay_block_number: u32) -> Option<H256>;

    #[cfg(feature = "runtime-benchmarks")]
    fn set_relay_storage_root(relay_block_number: u32, storage_root: Option<H256>);
}

impl RelayStorageRootProvider for () {
    fn get_relay_storage_root(_relay_block_number: u32) -> Option<H256> {
        None
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn set_relay_storage_root(_relay_block_number: u32, _storage_root: Option<H256>) {}
}

/// Information extracted from the latest container chain header
#[derive(
    Default,
    Clone,
    Encode,
    Decode,
    PartialEq,
    sp_core::RuntimeDebug,
    scale_info::TypeInfo,
    MaxEncodedLen,
    Serialize,
    Deserialize,
)]
pub struct ContainerChainBlockInfo<AccountId> {
    pub block_number: BlockNumber,
    pub author: AccountId,
    pub latest_slot_number: Slot,
}

pub trait LatestAuthorInfoFetcher<AccountId> {
    fn get_latest_author_info(para_id: ParaId) -> Option<ContainerChainBlockInfo<AccountId>>;
}

pub trait StorageDeposit<Data, Balance> {
    fn compute_deposit(data: &Data) -> Result<Balance, DispatchErrorWithPostInfo>;
}

pub struct BytesDeposit<BaseCost, ByteCost>(PhantomData<(BaseCost, ByteCost)>);
impl<Data, Balance, BaseCost, ByteCost> StorageDeposit<Data, Balance>
    for BytesDeposit<BaseCost, ByteCost>
where
    Data: Encode,
    Balance: TryFrom<usize> + CheckedAdd + CheckedMul,
    BaseCost: Get<Balance>,
    ByteCost: Get<Balance>,
{
    fn compute_deposit(data: &Data) -> Result<Balance, DispatchErrorWithPostInfo> {
        let base = BaseCost::get();
        let byte = ByteCost::get();
        let size: Balance = data
            .encoded_size()
            .try_into()
            .map_err(|_| ArithmeticError::Overflow)?;

        let deposit = byte
            .checked_mul(&size)
            .ok_or(ArithmeticError::Overflow)?
            .checked_add(&base)
            .ok_or(ArithmeticError::Overflow)?;

        Ok(deposit)
    }
}

/// Trait to abstract away relay storage proofs, and allow the same logic to work on both parachains and solochains.
/// Parachains should use relay storage proofs, while solochains should read from storage directly.
pub trait GenericStorageReader {
    fn read_entry<T: Decode>(&self, key: &[u8], fallback: Option<T>) -> Result<T, ReadEntryErr>;
}

impl GenericStorageReader for GenericStateProof<cumulus_primitives_core::relay_chain::Block> {
    fn read_entry<T: Decode>(&self, key: &[u8], fallback: Option<T>) -> Result<T, ReadEntryErr> {
        GenericStateProof::read_entry(self, key, fallback)
    }
}

/// Solo chain impl, read directly from storage
pub struct NativeStorageReader;
impl GenericStorageReader for NativeStorageReader {
    fn read_entry<T: Decode>(&self, key: &[u8], fallback: Option<T>) -> Result<T, ReadEntryErr> {
        match frame_support::storage::unhashed::get(key).or(fallback) {
            Some(x) => Ok(x),
            None => Err(ReadEntryErr::Absent),
        }
    }
}

/// Trait to handle registrar-related operations in a relay-chain context.
/// Mostly used to wire Tanssi's and Polkadot's registrars, for them to
/// work together in a solo-chain environment.
pub trait RegistrarHandler<AccountId> {
    fn register(
        who: AccountId,
        id: ParaId,
        genesis_storage: &[ContainerChainGenesisDataItem],
        head_data: Option<HeadData>,
    ) -> DispatchResult;

    fn schedule_para_upgrade(id: ParaId) -> DispatchResult;
    fn schedule_para_downgrade(id: ParaId) -> DispatchResult;
    fn deregister(id: ParaId);
    fn deregister_weight() -> Weight;

    #[cfg(feature = "runtime-benchmarks")]
    fn bench_head_data() -> Option<HeadData> {
        None
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn add_trusted_validation_code(_code: Vec<u8>) {}
    #[cfg(feature = "runtime-benchmarks")]
    fn registrar_new_session(_session: u32) {}
    #[cfg(feature = "runtime-benchmarks")]
    fn prepare_chain_registration(_id: ParaId, _who: AccountId) {}
}

impl<AccountId> RegistrarHandler<AccountId> for () {
    fn register(
        _who: AccountId,
        _id: ParaId,
        _genesis_storage: &[ContainerChainGenesisDataItem],
        _head_data: Option<HeadData>,
    ) -> DispatchResult {
        Ok(())
    }

    fn schedule_para_upgrade(_id: ParaId) -> DispatchResult {
        Ok(())
    }

    fn schedule_para_downgrade(_id: ParaId) -> DispatchResult {
        Ok(())
    }

    fn deregister(_id: ParaId) {}

    fn deregister_weight() -> Weight {
        Weight::default()
    }
}

/// Trait to retrieve the orchestrator block author (if any).
/// In a relay-chain context we will return None.
pub trait MaybeSelfChainBlockAuthor<AccountId> {
    fn get_block_author() -> Option<AccountId>;
}

impl<AccountId> MaybeSelfChainBlockAuthor<AccountId> for () {
    fn get_block_author() -> Option<AccountId> {
        None
    }
}

/// Information regarding the active era (era in used in session).
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ActiveEraInfo {
    /// Index of era.
    pub index: EraIndex,
    /// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
    ///
    /// Start can be none if start hasn't been set for the era yet,
    /// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
    pub start: Option<u64>,
}

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

pub trait EraIndexProvider {
    fn active_era() -> ActiveEraInfo;
    fn era_to_session_start(era_index: EraIndex) -> Option<u32>;
}

pub trait ValidatorProvider<ValidatorId> {
    fn validators() -> Vec<ValidatorId>;
}

pub trait InvulnerablesProvider<ValidatorId> {
    fn invulnerables() -> Vec<ValidatorId>;
}

pub trait OnEraStart {
    fn on_era_start(_era_index: EraIndex, _session_start: u32, _external_idx: u64) {}
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl OnEraStart for Tuple {
    fn on_era_start(era_index: EraIndex, session_start: u32, external_idx: u64) {
        for_tuples!( #( Tuple::on_era_start(era_index, session_start, external_idx); )* );
    }
}

pub trait OnEraEnd {
    fn on_era_end(_era_index: EraIndex) {}
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl OnEraEnd for Tuple {
    fn on_era_end(era_index: EraIndex) {
        for_tuples!( #( Tuple::on_era_end(era_index); )* );
    }
}

/// Strategy to use when rotating collators. Default: rotate all of them. Allows to rotate only a random subset.
#[derive(
    Clone,
    Debug,
    Default,
    Encode,
    Decode,
    scale_info::TypeInfo,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    MaxEncodedLen,
)]
pub enum FullRotationMode {
    #[default]
    RotateAll,
    KeepAll,
    /// Keep this many collators
    KeepCollators {
        keep: u32,
    },
    /// Keep a ratio of collators wrt to max collators.
    /// If max collators changes, the number of collators kept also changes.
    KeepPerbill {
        percentage: Perbill,
    },
}

/// Allow to set a different [FullRotationMode] for each kind of chain. Default: rotate all.
#[derive(
    Clone,
    Debug,
    Default,
    Encode,
    Decode,
    scale_info::TypeInfo,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    MaxEncodedLen,
)]
pub struct FullRotationModes {
    pub orchestrator: FullRotationMode,
    pub parachain: FullRotationMode,
    pub parathread: FullRotationMode,
}

impl FullRotationModes {
    /// Keep all collators assigned to their current chain if possible. This is equivalent to disabling rotation.
    pub fn keep_all() -> Self {
        Self {
            orchestrator: FullRotationMode::KeepAll,
            parachain: FullRotationMode::KeepAll,
            parathread: FullRotationMode::KeepAll,
        }
    }
}

// A trait to retrieve the external index provider identifying some set of data
// In starlight, used to retrieve the external index associated to validators
pub trait ExternalIndexProvider {
    fn get_external_index() -> u64;
}

// A trait to verify if a node has been inactive during the last minimum activity
pub trait NodeActivityTrackingHelper<AccountId> {
    fn is_node_inactive(node: &AccountId) -> bool;
}

// A trait to help verify if a ParaId is a chain or parathread
pub trait ParathreadHelper {
    fn get_parathreads_for_session() -> BTreeSet<ParaId>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_frequency_should_parathread_buy_core() {
        let slot_freq = SlotFrequency { min: 10, max: 20 };

        // Test case 1: Should buy core when current slot meets minimum requirement
        assert!(slot_freq.should_parathread_buy_core(Slot::from(15), Slot::from(2), Slot::from(5)));

        // Test case 2: Should not buy core when current slot is too early
        assert!(!slot_freq.should_parathread_buy_core(
            Slot::from(10),
            Slot::from(2),
            Slot::from(5)
        ));

        // Test case 3: Edge case with zero last_block_slot
        assert!(slot_freq.should_parathread_buy_core(Slot::from(8), Slot::from(2), Slot::from(0)));

        // Test case 4: Large max_slot_required_to_complete_purchase
        // With min=10, current=20, last=10, max_required=15
        // Should buy if: 20 >= 10 + 10 - 15 = 5 (true)
        assert!(slot_freq.should_parathread_buy_core(
            Slot::from(20),
            Slot::from(15),
            Slot::from(10)
        ));

        // Test case 5: Edge case with saturation
        // With min=10, current=5, last=10, max_required=20
        // Should buy if: 5 >= 10 + 10 - 20 = 0 (saturating_sub)
        // This evaluates to 5 >= 0 which is true
        assert!(slot_freq.should_parathread_buy_core(
            Slot::from(5),
            Slot::from(20),
            Slot::from(10)
        ));

        // Test case 6: Boundary condition
        let slot_freq_boundary = SlotFrequency { min: 1, max: 1 };
        assert!(slot_freq_boundary.should_parathread_buy_core(
            Slot::from(1),
            Slot::from(1),
            Slot::from(0)
        ));
    }

    #[test]
    fn test_slot_frequency_should_parathread_author_block() {
        let slot_freq = SlotFrequency { min: 10, max: 20 };

        // Test case 1: Should author block when minimum slots have passed
        assert!(slot_freq.should_parathread_author_block(Slot::from(15), Slot::from(5)));

        // Test case 2: Should not author block when not enough slots have passed
        assert!(!slot_freq.should_parathread_author_block(Slot::from(10), Slot::from(5)));

        // Test case 3: Exact boundary - should author
        assert!(slot_freq.should_parathread_author_block(Slot::from(15), Slot::from(5)));

        // Test case 4: Zero last_block_slot
        assert!(slot_freq.should_parathread_author_block(Slot::from(10), Slot::from(0)));

        // Test case 5: Overflow protection - very large values
        let large_slot = Slot::from(u64::MAX - 10);
        assert!(!slot_freq.should_parathread_author_block(Slot::from(5), large_slot));
    }

    #[test]
    fn test_slot_frequency_default() {
        let default_freq = SlotFrequency::default();
        assert_eq!(default_freq.min, 1);
        assert_eq!(default_freq.max, 1);

        // Default should allow authoring every slot
        assert!(default_freq.should_parathread_author_block(Slot::from(1), Slot::from(0)));
    }

    #[test]
    fn test_slot_frequency_edge_cases() {
        // Test with min > max (invalid but should handle gracefully)
        let invalid_freq = SlotFrequency { min: 20, max: 10 };

        // Should still work based on min value
        assert!(invalid_freq.should_parathread_author_block(Slot::from(25), Slot::from(5)));

        // Test with zero values
        let zero_freq = SlotFrequency { min: 0, max: 0 };
        assert!(zero_freq.should_parathread_author_block(Slot::from(0), Slot::from(0)));

        // Test with very large values
        let large_freq = SlotFrequency {
            min: u32::MAX,
            max: u32::MAX,
        };
        assert!(!large_freq.should_parathread_author_block(Slot::from(1000), Slot::from(0)));
    }

    #[test]
    fn test_bytes_deposit_compute_deposit() {
        use frame_support::parameter_types;

        parameter_types! {
            pub const BaseDeposit: u128 = 100;
            pub const ByteDeposit: u128 = 10;
        }

        // Test case 1: Simple data with known size
        let data = vec![1u8, 2, 3, 4];
        let deposit: u128 =
            BytesDeposit::<BaseDeposit, ByteDeposit>::compute_deposit(&data).unwrap();
        // Vec encoding: 1 byte compact length + 4 bytes data = 5 bytes total
        // 5 bytes * 10 per byte + 100 base = 150
        assert_eq!(deposit, 150u128);

        // Test case 2: Empty data
        let empty_data: Vec<u8> = Vec::new();
        let deposit: u128 =
            BytesDeposit::<BaseDeposit, ByteDeposit>::compute_deposit(&empty_data).unwrap();
        // Vec encoding: 1 byte compact length (0) = 1 byte total
        // 1 byte * 10 + 100 base = 110
        assert_eq!(deposit, 110u128);

        // Test case 3: Larger data
        let large_data = vec![0u8; 1000];
        let deposit: u128 =
            BytesDeposit::<BaseDeposit, ByteDeposit>::compute_deposit(&large_data).unwrap();
        // Vec encoding: 2 bytes compact length + 1000 bytes data = 1002 bytes total
        // 1002 bytes * 10 + 100 base = 10120
        assert_eq!(deposit, 10120u128);

        // Test case 4: Complex struct
        #[derive(Encode)]
        struct TestStruct {
            id: u32,
            data: Vec<u8>,
        }

        let test_struct = TestStruct {
            id: 42,
            data: vec![1, 2, 3],
        };
        let deposit: u128 =
            BytesDeposit::<BaseDeposit, ByteDeposit>::compute_deposit(&test_struct).unwrap();
        // id (4 bytes) + compact encoding of vec length + 3 bytes = ~8 bytes * 10 + 100 = ~180
        assert!((180u128..=200u128).contains(&deposit));
    }

    #[test]
    fn test_bytes_deposit_overflow_protection() {
        use frame_support::parameter_types;

        parameter_types! {
            pub const BaseDepositOverflow: u128 = u128::MAX - 100;
            pub const ByteDepositOverflow: u128 = 100;
        }

        // Test case 1: Base overflow
        let small_data = vec![1u8];
        let result: Result<u128, DispatchErrorWithPostInfo> =
            BytesDeposit::<BaseDepositOverflow, ByteDepositOverflow>::compute_deposit(&small_data);
        assert!(result.is_err());

        // Test case 2: Multiplication overflow
        parameter_types! {
            pub const BaseDepositSafe: u128 = 100;
            pub const ByteDepositLarge: u128 = u128::MAX / 2;
        }

        let data = vec![0u8; 3];
        let result: Result<u128, DispatchErrorWithPostInfo> =
            BytesDeposit::<BaseDepositSafe, ByteDepositLarge>::compute_deposit(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_deposit_edge_cases() {
        use frame_support::parameter_types;

        parameter_types! {
            pub const ZeroBase: u128 = 0;
            pub const ZeroByte: u128 = 0;
        }

        // Test with zero costs
        let data = vec![1u8, 2, 3];
        let deposit: u128 = BytesDeposit::<ZeroBase, ZeroByte>::compute_deposit(&data).unwrap();
        assert_eq!(deposit, 0u128);

        parameter_types! {
            pub const OneBase: u128 = 1;
            pub const OneByte: u128 = 1;
        }

        // Test with minimum non-zero costs
        let data = vec![1u8];
        let deposit: u128 = BytesDeposit::<OneBase, OneByte>::compute_deposit(&data).unwrap();
        // Vec encoding: 1 byte length + 1 byte data = 2 bytes
        // 2 bytes * 1 + 1 base = 3
        assert_eq!(deposit, 3u128);
    }

    #[test]
    fn test_full_rotation_mode() {
        // Test default
        let default_mode = FullRotationMode::default();
        assert_eq!(default_mode, FullRotationMode::RotateAll);

        // Test all variants
        let rotate_all = FullRotationMode::RotateAll;
        let keep_all = FullRotationMode::KeepAll;
        let keep_collators = FullRotationMode::KeepCollators { keep: 10 };
        let keep_perbill = FullRotationMode::KeepPerbill {
            percentage: Perbill::from_percent(50),
        };

        // Test equality
        assert_eq!(rotate_all, FullRotationMode::RotateAll);
        assert_eq!(keep_all, FullRotationMode::KeepAll);
        assert_eq!(keep_collators, FullRotationMode::KeepCollators { keep: 10 });
        assert_ne!(keep_collators, FullRotationMode::KeepCollators { keep: 20 });

        // Test encoding/decoding
        let encoded_rotate = rotate_all.encode();
        let decoded_rotate = FullRotationMode::decode(&mut &encoded_rotate[..]).unwrap();
        assert_eq!(decoded_rotate, rotate_all);

        let encoded_keep = keep_collators.encode();
        let decoded_keep = FullRotationMode::decode(&mut &encoded_keep[..]).unwrap();
        assert_eq!(decoded_keep, keep_collators);

        let encoded_perbill = keep_perbill.encode();
        let decoded_perbill = FullRotationMode::decode(&mut &encoded_perbill[..]).unwrap();
        assert_eq!(decoded_perbill, keep_perbill);
    }

    #[test]
    fn test_full_rotation_modes() {
        // Test default
        let default_modes = FullRotationModes::default();
        assert_eq!(default_modes.orchestrator, FullRotationMode::RotateAll);
        assert_eq!(default_modes.parachain, FullRotationMode::RotateAll);
        assert_eq!(default_modes.parathread, FullRotationMode::RotateAll);

        // Test keep_all() method
        let keep_all_modes = FullRotationModes::keep_all();
        assert_eq!(keep_all_modes.orchestrator, FullRotationMode::KeepAll);
        assert_eq!(keep_all_modes.parachain, FullRotationMode::KeepAll);
        assert_eq!(keep_all_modes.parathread, FullRotationMode::KeepAll);

        // Test custom configuration
        let custom_modes = FullRotationModes {
            orchestrator: FullRotationMode::KeepCollators { keep: 5 },
            parachain: FullRotationMode::KeepPerbill {
                percentage: Perbill::from_percent(75),
            },
            parathread: FullRotationMode::RotateAll,
        };

        assert_eq!(
            custom_modes.orchestrator,
            FullRotationMode::KeepCollators { keep: 5 }
        );
        assert_eq!(
            custom_modes.parachain,
            FullRotationMode::KeepPerbill {
                percentage: Perbill::from_percent(75)
            }
        );
        assert_eq!(custom_modes.parathread, FullRotationMode::RotateAll);

        // Test encoding/decoding
        let encoded = custom_modes.encode();
        let decoded = FullRotationModes::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded.orchestrator, custom_modes.orchestrator);
        assert_eq!(decoded.parachain, custom_modes.parachain);
        assert_eq!(decoded.parathread, custom_modes.parathread);
    }

    #[test]
    fn test_full_rotation_mode_edge_cases() {
        // Test extreme values
        let zero_keep = FullRotationMode::KeepCollators { keep: 0 };
        let max_keep = FullRotationMode::KeepCollators { keep: u32::MAX };

        assert_eq!(zero_keep, FullRotationMode::KeepCollators { keep: 0 });
        assert_eq!(max_keep, FullRotationMode::KeepCollators { keep: u32::MAX });

        // Test perbill edge cases
        let zero_percent = FullRotationMode::KeepPerbill {
            percentage: Perbill::from_percent(0),
        };
        let hundred_percent = FullRotationMode::KeepPerbill {
            percentage: Perbill::from_percent(100),
        };
        let one_perbill = FullRotationMode::KeepPerbill {
            percentage: Perbill::from_parts(1),
        };

        assert_eq!(
            zero_percent,
            FullRotationMode::KeepPerbill {
                percentage: Perbill::from_percent(0)
            }
        );
        assert_eq!(
            hundred_percent,
            FullRotationMode::KeepPerbill {
                percentage: Perbill::from_percent(100)
            }
        );
        assert_eq!(
            one_perbill,
            FullRotationMode::KeepPerbill {
                percentage: Perbill::from_parts(1)
            }
        );
    }

    #[test]
    fn test_author_noting_info() {
        // Create mock account ID type
        #[allow(dead_code)]
        type AccountId = u64;

        // Test creation
        let info = AuthorNotingInfo {
            author: 42u64,
            block_number: 100,
            para_id: ParaId::from(2000),
        };

        assert_eq!(info.author, 42u64);
        assert_eq!(info.block_number, 100);
        assert_eq!(info.para_id, ParaId::from(2000));

        // Test with different values
        let info2 = AuthorNotingInfo {
            author: 0u64,
            block_number: u32::MAX,
            para_id: ParaId::from(0),
        };

        assert_eq!(info2.author, 0u64);
        assert_eq!(info2.block_number, u32::MAX);
        assert_eq!(info2.para_id, ParaId::from(0));

        // Test array of infos (as used in the trait)
        let infos = vec![
            AuthorNotingInfo {
                author: 1u64,
                block_number: 10,
                para_id: ParaId::from(1000),
            },
            AuthorNotingInfo {
                author: 2u64,
                block_number: 20,
                para_id: ParaId::from(2000),
            },
        ];

        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].author, 1u64);
        assert_eq!(infos[1].para_id, ParaId::from(2000));
    }

    #[test]
    fn test_parathread_params() {
        // Test with default slot frequency
        let params1 = ParathreadParams {
            slot_frequency: SlotFrequency::default(),
        };

        assert_eq!(params1.slot_frequency.min, 1);
        assert_eq!(params1.slot_frequency.max, 1);

        // Test with custom slot frequency
        let params2 = ParathreadParams {
            slot_frequency: SlotFrequency { min: 5, max: 10 },
        };

        assert_eq!(params2.slot_frequency.min, 5);
        assert_eq!(params2.slot_frequency.max, 10);

        // Test encoding/decoding
        let encoded = params2.encode();
        let decoded = ParathreadParams::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded.slot_frequency.min, 5);
        assert_eq!(decoded.slot_frequency.max, 10);
    }

    #[test]
    fn test_container_chain_block_info() {
        type AccountId = u64;

        // Test default
        let default_info: ContainerChainBlockInfo<AccountId> = Default::default();
        assert_eq!(default_info.block_number, 0);
        assert_eq!(default_info.author, 0u64);
        assert_eq!(default_info.latest_slot_number, Slot::from(0));

        // Test custom values
        let info = ContainerChainBlockInfo {
            block_number: 12345,
            author: 999u64,
            latest_slot_number: Slot::from(100),
        };

        assert_eq!(info.block_number, 12345);
        assert_eq!(info.author, 999u64);
        assert_eq!(info.latest_slot_number, Slot::from(100));

        // Test encoding/decoding
        let encoded = info.encode();
        let decoded = ContainerChainBlockInfo::<AccountId>::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded.block_number, info.block_number);
        assert_eq!(decoded.author, info.author);
        assert_eq!(decoded.latest_slot_number, info.latest_slot_number);

        // Test edge cases
        let edge_info = ContainerChainBlockInfo {
            block_number: u32::MAX,
            author: u64::MAX,
            latest_slot_number: Slot::from(u64::MAX),
        };

        assert_eq!(edge_info.block_number, u32::MAX);
        assert_eq!(edge_info.author, u64::MAX);
        assert_eq!(edge_info.latest_slot_number, Slot::from(u64::MAX));
    }

    #[test]
    fn test_session_container_chains() {
        // Test empty
        let empty_chains = SessionContainerChains {
            parachains: vec![],
            parathreads: vec![],
        };

        assert!(empty_chains.parachains.is_empty());
        assert!(empty_chains.parathreads.is_empty());

        // Test with data
        let chains = SessionContainerChains {
            parachains: vec![ParaId::from(1000), ParaId::from(2000)],
            parathreads: vec![
                (
                    ParaId::from(3000),
                    ParathreadParams {
                        slot_frequency: SlotFrequency { min: 5, max: 10 },
                    },
                ),
                (
                    ParaId::from(4000),
                    ParathreadParams {
                        slot_frequency: SlotFrequency { min: 1, max: 1 },
                    },
                ),
            ],
        };

        assert_eq!(chains.parachains.len(), 2);
        assert_eq!(chains.parathreads.len(), 2);
        assert_eq!(chains.parachains[0], ParaId::from(1000));
        assert_eq!(chains.parathreads[0].0, ParaId::from(3000));
        assert_eq!(chains.parathreads[0].1.slot_frequency.min, 5);

        // Test encoding/decoding
        let encoded = chains.encode();
        let decoded = SessionContainerChains::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded.parachains.len(), chains.parachains.len());
        assert_eq!(decoded.parathreads.len(), chains.parathreads.len());
        assert_eq!(decoded.parachains[1], ParaId::from(2000));
        assert_eq!(decoded.parathreads[1].1.slot_frequency.max, 1);
    }

    #[test]
    fn test_active_era_info() {
        // Test with start time
        let era_info = ActiveEraInfo {
            index: 42,
            start: Some(1234567890),
        };

        assert_eq!(era_info.index, 42);
        assert_eq!(era_info.start, Some(1234567890));

        // Test without start time
        let era_info_no_start = ActiveEraInfo {
            index: 0,
            start: None,
        };

        assert_eq!(era_info_no_start.index, 0);
        assert_eq!(era_info_no_start.start, None);

        // Test encoding/decoding
        let encoded = era_info.encode();
        let decoded = ActiveEraInfo::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded.index, 42);
        assert_eq!(decoded.start, Some(1234567890));

        // Test edge cases
        let edge_era = ActiveEraInfo {
            index: u32::MAX,
            start: Some(u64::MAX),
        };

        assert_eq!(edge_era.index, u32::MAX);
        assert_eq!(edge_era.start, Some(u64::MAX));
    }

    #[test]
    fn test_for_session_enum() {
        // Test equality
        assert_eq!(ForSession::Current, ForSession::Current);
        assert_eq!(ForSession::Next, ForSession::Next);
        assert_ne!(ForSession::Current, ForSession::Next);

        // Test pattern matching
        let session = ForSession::Current;
        match session {
            ForSession::Current => {}
            ForSession::Next => panic!("Should be Current"),
        }

        let session = ForSession::Next;
        match session {
            ForSession::Current => panic!("Should be Next"),
            ForSession::Next => {}
        }
    }

    #[test]
    #[allow(unsafe_code)]
    #[ignore = "Requires runtime environment"]
    fn test_native_storage_reader() {
        use frame_support::storage::unhashed;

        // Test reading non-existent key with no fallback
        let reader = NativeStorageReader;
        let result: Result<u32, ReadEntryErr> = reader.read_entry(b"non_existent_key", None);
        assert!(matches!(result, Err(ReadEntryErr::Absent)));

        // Test reading non-existent key with fallback
        let result: Result<u32, ReadEntryErr> = reader.read_entry(b"non_existent_key", Some(42u32));
        assert!(matches!(result, Ok(42u32)));

        // Test storing and reading a value
        unhashed::put(b"test_key", &123u64);

        let result: Result<u64, ReadEntryErr> = reader.read_entry(b"test_key", None);
        assert!(matches!(result, Ok(123u64)));

        // Test reading with wrong type should fail to decode
        let result: Result<String, ReadEntryErr> = reader.read_entry(b"test_key", None);
        assert!(result.is_err()); // Will fail to decode u64 as String

        // Test reading with fallback when value exists
        let result: Result<u64, ReadEntryErr> = reader.read_entry(b"test_key", Some(999u64));
        assert!(matches!(result, Ok(123u64))); // Should return stored value, not fallback

        // Clean up
        unhashed::kill(b"test_key");

        // Test complex types
        #[derive(Encode, Decode, PartialEq, Debug)]
        struct TestStruct {
            id: u32,
            name: Vec<u8>,
        }

        let test_data = TestStruct {
            id: 456,
            name: b"test".to_vec(),
        };

        unhashed::put(b"complex_key", &test_data);

        let result: Result<TestStruct, ReadEntryErr> = reader.read_entry(b"complex_key", None);
        assert!(matches!(result, Ok(ref data) if data == &test_data));

        // Clean up
        unhashed::kill(b"complex_key");
    }

    #[test]
    #[allow(unsafe_code)]
    #[ignore = "Requires runtime environment"]
    fn test_native_storage_reader_edge_cases() {
        use frame_support::storage::unhashed;
        let reader = NativeStorageReader;

        // Test empty key
        let result: Result<u32, ReadEntryErr> = reader.read_entry(b"", None);
        assert!(matches!(result, Err(ReadEntryErr::Absent)));

        // Test very long key
        let long_key = vec![b'a'; 1000];
        let result: Result<u32, ReadEntryErr> = reader.read_entry(&long_key, None);
        assert!(matches!(result, Err(ReadEntryErr::Absent)));

        // Test Option type
        unhashed::put(b"option_some", &Some(42u32));
        unhashed::put(b"option_none", &None::<u32>);

        let result: Result<Option<u32>, ReadEntryErr> = reader.read_entry(b"option_some", None);
        assert!(matches!(result, Ok(Some(42u32))));

        let result: Result<Option<u32>, ReadEntryErr> = reader.read_entry(b"option_none", None);
        assert!(matches!(result, Ok(None)));

        // Clean up
        unhashed::kill(b"option_some");
        unhashed::kill(b"option_none");
    }

    #[test]
    fn test_prod_or_fast_parameter_types_macro() {
        use crate::prod_or_fast_parameter_types;

        // Test with const parameters
        prod_or_fast_parameter_types! {
            pub const TestBlockTime: u64 = { prod: 6000, fast: 1000 };
        }

        // Test the methods
        assert_eq!(TestBlockTime::prod(), 6000);
        assert_eq!(TestBlockTime::fast(), 1000);
        assert_eq!(TestBlockTime::prod_if(true), 6000);
        assert_eq!(TestBlockTime::prod_if(false), 1000);

        // Test Get implementation
        let value: u64 = <TestBlockTime as Get<u64>>::get();
        #[cfg(feature = "fast-runtime")]
        assert_eq!(value, 1000);
        #[cfg(not(feature = "fast-runtime"))]
        assert_eq!(value, 6000);

        // Test with non-const parameters (different macro branch)
        prod_or_fast_parameter_types! {
            pub TestComplexValue: Vec<u8> = { prod: vec![1, 2, 3], fast: vec![4, 5] };
        }

        assert_eq!(TestComplexValue::prod(), vec![1, 2, 3]);
        assert_eq!(TestComplexValue::fast(), vec![4, 5]);

        // Test multiple parameters in one macro call
        prod_or_fast_parameter_types! {
            pub const TestValue1: u32 = { prod: 100, fast: 10 };
            pub const TestValue2: u128 = { prod: 1_000_000, fast: 1_000 };
            pub TestString: &'static str = { prod: "production", fast: "fast" };
        }

        assert_eq!(TestValue1::prod(), 100);
        assert_eq!(TestValue1::fast(), 10);
        assert_eq!(TestValue2::prod(), 1_000_000);
        assert_eq!(TestValue2::fast(), 1_000);
        assert_eq!(TestString::prod(), "production");
        assert_eq!(TestString::fast(), "fast");

        // Test Get trait conversion
        let val1: u32 = TestValue1::get();
        let val2: u128 = TestValue2::get();
        #[cfg(feature = "fast-runtime")]
        #[allow(dead_code)]
        {
            assert_eq!(val1, 10);
            assert_eq!(val2, 1_000);
        }
        #[cfg(not(feature = "fast-runtime"))]
        #[allow(dead_code)]
        {
            assert_eq!(val1, 100);
            assert_eq!(val2, 1_000_000);
        }
    }
}

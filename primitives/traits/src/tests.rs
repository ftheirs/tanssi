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

//! Unit tests for external dependencies from Dancekit and local traits

use {
    crate::{
        AuthorNotingHook, AuthorNotingInfo, CollatorAssignmentHook, CollatorAssignmentTip,
        ContainerChainBlockInfo, ContainerChainGenesisDataItem, DistributeRewards,
        ForSession, ParaIdAssignmentHooks, ParathreadParams, RelayStorageRootProvider, 
        RemoveInvulnerables, SessionContainerChains, ShouldRotateAllCollators, SlotFrequency, 
        StorageDeposit, BytesDeposit,
    },
    cumulus_primitives_core::{relay_chain::Slot, ParaId},
    dp_chain_state_snapshot::ReadEntryErr,
    frame_support::{
        pallet_prelude::{Get, Weight},
    },
    parity_scale_codec::{Decode, Encode},
    sp_runtime::DispatchError,
    sp_std::{
        collections::{btree_map::BTreeMap, btree_set::BTreeSet},
        vec,
    },
};

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
struct TestData {
    value: u32,
}

#[test]
fn test_container_chain_genesis_data_item_encoding() {
    // Test encoding and decoding of ContainerChainGenesisDataItem
    let key = b"test_key".to_vec();
    let value = b"test_value".to_vec();
    
    let item = ContainerChainGenesisDataItem {
        key: key.clone(),
        value: value.clone(),
    };
    
    // Test encoding
    let encoded = item.encode();
    
    // Test decoding
    let decoded = ContainerChainGenesisDataItem::decode(&mut &encoded[..]).unwrap();
    
    assert_eq!(decoded.key, key);
    assert_eq!(decoded.value, value);
}

#[test]
fn test_container_chain_genesis_data_item_from_tuple() {
    // Test conversion from tuple
    let key = b"test_key".to_vec();
    let value = b"test_value".to_vec();
    
    let item: ContainerChainGenesisDataItem = (key.clone(), value.clone()).into();
    
    assert_eq!(item.key, key);
    assert_eq!(item.value, value);
}

// Tests for GenericStorageReader trait implementation are complex because they require
// setting up state proofs. These are better tested through integration tests
// in the actual usage context (e.g., in pallet-registrar tests)

#[cfg(test)]
mod native_storage_reader_tests {
    use super::*;
    use crate::{NativeStorageReader, GenericStorageReader};
    use frame_support::storage::unhashed;
    
    #[test]
    fn test_native_storage_reader_read_existing() {
        // This test uses TestExternalities to simulate native storage
        sp_io::TestExternalities::default().execute_with(|| {
            let key = b"test_key";
            let test_data = TestData { value: 456 };
            
            // Put data in storage
            unhashed::put(key, &test_data);
            
            // Read using NativeStorageReader
            let reader = NativeStorageReader;
            let result: TestData = reader.read_entry(key, None).unwrap();
            assert_eq!(result.value, 456);
        });
    }
    
    #[test]
    fn test_native_storage_reader_read_with_fallback() {
        sp_io::TestExternalities::default().execute_with(|| {
            let key = b"non_existent_key";
            let fallback = TestData { value: 789 };
            
            // Read non-existent key with fallback
            let reader = NativeStorageReader;
            let result: TestData = reader.read_entry(key, Some(fallback)).unwrap();
            assert_eq!(result.value, 789);
        });
    }
    
    #[test]
    fn test_native_storage_reader_read_absent() {
        sp_io::TestExternalities::default().execute_with(|| {
            let key = b"non_existent_key";
            
            // Read non-existent key without fallback
            let reader = NativeStorageReader;
            let result: Result<TestData, ReadEntryErr> = reader.read_entry(key, None);
            assert!(matches!(result, Err(ReadEntryErr::Absent)));
        });
    }
}

// Tests for SlotFrequency
#[test]
fn test_slot_frequency_default() {
    let freq = SlotFrequency::default();
    assert_eq!(freq.min, 1);
    assert_eq!(freq.max, 1);
}

#[test]
fn test_slot_frequency_should_parathread_buy_core() {
    let freq = SlotFrequency { min: 10, max: 20 };
    
    // Test case 1: Should buy core
    // last_block_slot + min - max_slot_required = 85 + 10 - 5 = 90
    // current_slot (95) >= 90, so should buy
    let current_slot = Slot::from(95);
    let max_slot_required = Slot::from(5);
    let last_block_slot = Slot::from(85);
    
    assert!(freq.should_parathread_buy_core(current_slot, max_slot_required, last_block_slot));
    
    // Test case 2: Should not buy core yet
    // last_block_slot + min - max_slot_required = 85 + 10 - 5 = 90
    // current_slot (89) < 90, so should not buy
    let current_slot = Slot::from(89);
    assert!(!freq.should_parathread_buy_core(current_slot, max_slot_required, last_block_slot));
}

#[test]
fn test_slot_frequency_should_parathread_author_block() {
    let freq = SlotFrequency { min: 10, max: 20 };
    
    // Test case 1: Should author block
    let current_slot = Slot::from(100);
    let last_block_slot = Slot::from(90);
    assert!(freq.should_parathread_author_block(current_slot, last_block_slot));
    
    // Test case 2: Should not author block yet
    let current_slot = Slot::from(95);
    assert!(!freq.should_parathread_author_block(current_slot, last_block_slot));
}

// Tests for AuthorNotingInfo and ContainerChainBlockInfo
#[test]
fn test_author_noting_info_creation() {
    let info = AuthorNotingInfo {
        author: 42u64,
        block_number: 100,
        para_id: ParaId::from(1000),
    };
    
    assert_eq!(info.author, 42u64);
    assert_eq!(info.block_number, 100);
    assert_eq!(info.para_id, ParaId::from(1000));
}

#[test]
fn test_container_chain_block_info_default() {
    let info: ContainerChainBlockInfo<u64> = Default::default();
    assert_eq!(info.block_number, 0);
    assert_eq!(info.author, 0u64);
    assert_eq!(info.latest_slot_number, Slot::from(0));
}

#[test]
fn test_container_chain_block_info_encoding() {
    let info = ContainerChainBlockInfo {
        block_number: 123,
        author: 456u64,
        latest_slot_number: Slot::from(789),
    };
    
    let encoded = info.encode();
    let decoded = ContainerChainBlockInfo::<u64>::decode(&mut &encoded[..]).unwrap();
    
    assert_eq!(decoded.block_number, 123);
    assert_eq!(decoded.author, 456u64);
    assert_eq!(decoded.latest_slot_number, Slot::from(789));
}

// Tests for ForSession enum
#[test]
fn test_for_session_equality() {
    assert_eq!(ForSession::Current, ForSession::Current);
    assert_eq!(ForSession::Next, ForSession::Next);
    assert_ne!(ForSession::Current, ForSession::Next);
}

// Tests for SessionContainerChains
#[test]
fn test_session_container_chains_encoding() {
    let params = ParathreadParams {
        slot_frequency: SlotFrequency { min: 5, max: 10 },
    };
    
    let chains = SessionContainerChains {
        parachains: vec![ParaId::from(1000), ParaId::from(2000)],
        parathreads: vec![
            (ParaId::from(3000), params.clone()),
            (ParaId::from(4000), params),
        ],
    };
    
    let encoded = chains.encode();
    let decoded = SessionContainerChains::decode(&mut &encoded[..]).unwrap();
    
    assert_eq!(decoded.parachains.len(), 2);
    assert_eq!(decoded.parathreads.len(), 2);
    assert_eq!(decoded.parachains[0], ParaId::from(1000));
    assert_eq!(decoded.parathreads[0].0, ParaId::from(3000));
    assert_eq!(decoded.parathreads[0].1.slot_frequency.min, 5);
}

// Tests for default trait implementations
#[test]
fn test_collator_assignment_tip_default() {
    let tip = <() as CollatorAssignmentTip<u128>>::get_para_tip(ParaId::from(1000));
    assert_eq!(tip, None);
}

#[test]
fn test_distribute_rewards_default() {
    struct MockImbalance;
    let result = <() as DistributeRewards<u64, MockImbalance>>::distribute_rewards(
        42u64,
        MockImbalance,
    );
    assert!(result.is_ok());
}

#[test]
fn test_should_rotate_all_collators_default() {
    let should_rotate = <() as ShouldRotateAllCollators<u32>>::should_rotate_all_collators(10);
    assert!(!should_rotate);
}

#[test]
fn test_remove_invulnerables_default() {
    let mut collators = vec![1u64, 2, 3, 4, 5];
    let invulnerables = <() as RemoveInvulnerables<u64>>::remove_invulnerables(&mut collators, 2);
    assert_eq!(invulnerables.len(), 0);
    assert_eq!(collators.len(), 5); // No change
}

#[test]
fn test_relay_storage_root_provider_default() {
    let root = <() as RelayStorageRootProvider>::get_relay_storage_root(100);
    assert_eq!(root, None);
}

// Tests for BytesDeposit
struct ConstU128<const N: u128>;
impl<const N: u128> Get<u128> for ConstU128<N> {
    fn get() -> u128 {
        N
    }
}

#[test]
fn test_bytes_deposit_compute() {
    type TestDeposit = BytesDeposit<ConstU128<100>, ConstU128<10>>;
    
    // Test with small data
    let small_data = vec![1u8, 2, 3];
    let deposit = TestDeposit::compute_deposit(&small_data).unwrap();
    // Vec encoding: compact length (1 byte for length 3) + 3 bytes = 4 bytes total
    // Base cost (100) + 4 bytes * 10 = 140
    assert_eq!(deposit, 140u128);
    
    // Test with larger data
    let large_data = vec![0u8; 50];
    let deposit = TestDeposit::compute_deposit(&large_data).unwrap();
    // Vec encoding: compact length (1 byte for length 50) + 50 bytes = 51 bytes total
    // Base cost (100) + 51 bytes * 10 = 610
    assert_eq!(deposit, 610u128);
}

// Tests for ParaIdAssignmentHooks default implementation
#[test]
fn test_para_id_assignment_hooks_default() {
    let mut para_ids = vec![ParaId::from(1000), ParaId::from(2000)];
    let old_assigned = BTreeSet::new();
    
    <() as ParaIdAssignmentHooks<u128, u64>>::pre_assignment(&mut para_ids, &old_assigned);
    assert_eq!(para_ids.len(), 2); // No change
    
    let current_assigned = BTreeSet::new();
    let mut new_assigned = BTreeMap::new();
    let tip = Some(100u128);
    
    let weight = <() as ParaIdAssignmentHooks<u128, u64>>::post_assignment(
        &current_assigned,
        &mut new_assigned,
        &tip,
    );
    assert_eq!(weight, Weight::zero());
}

// Mock implementations for testing trait tuple implementations
struct MockCollatorAssignmentHook;
impl CollatorAssignmentHook<u128> for MockCollatorAssignmentHook {
    fn on_collators_assigned(
        _para_id: ParaId,
        _maybe_tip: Option<&u128>,
        _is_parathread: bool,
    ) -> Result<Weight, DispatchError> {
        Ok(Weight::from_parts(100, 0))
    }
}

struct MockAuthorNotingHook;
impl<AccountId> AuthorNotingHook<AccountId> for MockAuthorNotingHook {
    fn on_container_authors_noted(_info: &[AuthorNotingInfo<AccountId>]) -> Weight {
        Weight::from_parts(200, 0)
    }
    
    #[cfg(feature = "runtime-benchmarks")]
    fn prepare_worst_case_for_bench(_author: &AccountId, _block_number: u32, _para_id: ParaId) {}
}

#[test]
fn test_collator_assignment_hook_tuple() {
    // Test tuple implementation
    type TupleHook = (MockCollatorAssignmentHook, MockCollatorAssignmentHook);
    
    let result = TupleHook::on_collators_assigned(
        ParaId::from(1000),
        Some(&100u128),
        false,
    );
    
    assert!(result.is_ok());
    // Should be 100 + 100 = 200
    assert_eq!(result.unwrap(), Weight::from_parts(200, 0));
}

#[test]
fn test_author_noting_hook_tuple() {
    // Test tuple implementation
    type TupleHook = (MockAuthorNotingHook, MockAuthorNotingHook);
    
    let info = vec![
        AuthorNotingInfo {
            author: 42u64,
            block_number: 100,
            para_id: ParaId::from(1000),
        },
    ];
    
    let weight = TupleHook::on_container_authors_noted(&info);
    // Should be 200 + 200 = 400
    assert_eq!(weight, Weight::from_parts(400, 0));
}
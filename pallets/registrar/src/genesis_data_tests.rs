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

//! Unit tests for ContainerChainGenesisData from external dependencies

use {
    dp_container_chain_genesis_data::{ContainerChainGenesisData, Properties},
    frame_support::BoundedVec,
    parity_scale_codec::{Decode, Encode},
};

#[test]
fn test_container_chain_genesis_data_encoding() {
    // Create test genesis data
    let mut storage = BoundedVec::try_from(vec![]).unwrap();
    storage.try_push((b"key1".to_vec(), b"value1".to_vec()).into()).unwrap();
    storage.try_push((b"key2".to_vec(), b"value2".to_vec()).into()).unwrap();
    
    let genesis_data = ContainerChainGenesisData {
        storage: storage.clone(),
        name: b"test-chain".to_vec().try_into().unwrap(),
        id: b"test-id".to_vec().try_into().unwrap(),
        fork_id: Some(b"test-fork".to_vec().try_into().unwrap()),
        extensions: b"test-ext".to_vec().try_into().unwrap(),
        properties: Properties {
            token_metadata: Default::default(),
            is_ethereum: false,
        },
    };
    
    // Test encoding
    let encoded = genesis_data.encode();
    
    // Test decoding
    let decoded = ContainerChainGenesisData::decode(&mut &encoded[..]).unwrap();
    
    assert_eq!(decoded.storage, storage);
    assert_eq!(decoded.name, genesis_data.name);
    assert_eq!(decoded.id, genesis_data.id);
    assert_eq!(decoded.fork_id, genesis_data.fork_id);
    assert_eq!(decoded.extensions, genesis_data.extensions);
    assert_eq!(decoded.properties.is_ethereum, false);
}

#[test]
fn test_container_chain_genesis_data_default() {
    let genesis_data = ContainerChainGenesisData::default();
    
    assert!(genesis_data.storage.is_empty());
    assert!(genesis_data.name.is_empty());
    assert!(genesis_data.id.is_empty());
    assert!(genesis_data.fork_id.is_none());
    assert!(genesis_data.extensions.is_empty());
    assert!(!genesis_data.properties.is_ethereum);
}

#[test]
fn test_container_chain_genesis_data_with_ethereum_properties() {
    let mut storage = BoundedVec::try_from(vec![]).unwrap();
    storage.try_push((b":code".to_vec(), vec![1, 2, 3]).into()).unwrap();
    
    let genesis_data = ContainerChainGenesisData {
        storage,
        name: b"ethereum-chain".to_vec().try_into().unwrap(),
        id: b"eth-1".to_vec().try_into().unwrap(),
        fork_id: None,
        extensions: Default::default(),
        properties: Properties {
            token_metadata: Default::default(),
            is_ethereum: true,
        },
    };
    
    assert!(genesis_data.properties.is_ethereum);
}

#[test]
fn test_container_chain_genesis_data_storage_limits() {
    // Test with maximum allowed storage items
    let mut storage = BoundedVec::try_from(vec![]).unwrap();
    
    // Add items until we reach the bound (this will depend on the actual bound in the type)
    for i in 0..100 {
        let key = format!("key{}", i).into_bytes();
        let value = format!("value{}", i).into_bytes();
        match storage.try_push((key, value).into()) {
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    
    let genesis_data = ContainerChainGenesisData {
        storage,
        name: Default::default(),
        id: Default::default(),
        fork_id: None,
        extensions: Default::default(),
        properties: Default::default(),
    };
    
    // Ensure we can encode and decode even with many items
    let encoded = genesis_data.encode();
    let decoded = ContainerChainGenesisData::decode(&mut &encoded[..]).unwrap();
    assert_eq!(decoded.storage.len(), genesis_data.storage.len());
}

#[test]
fn test_genesis_data_item_ordering() {
    // Test that storage items maintain their order
    let items = vec![
        (b"z_last".to_vec(), b"value1".to_vec()),
        (b"a_first".to_vec(), b"value2".to_vec()),
        (b"m_middle".to_vec(), b"value3".to_vec()),
    ];
    
    let mut storage = BoundedVec::try_from(vec![]).unwrap();
    for item in &items {
        storage.try_push(item.clone().into()).unwrap();
    }
    
    let genesis_data = ContainerChainGenesisData {
        storage: storage.clone(),
        name: Default::default(),
        id: Default::default(),
        fork_id: None,
        extensions: Default::default(),
        properties: Default::default(),
    };
    
    // Verify order is preserved
    for (i, item) in genesis_data.storage.iter().enumerate() {
        assert_eq!(item.key, items[i].0);
        assert_eq!(item.value, items[i].1);
    }
}

#[test]
fn test_properties_token_metadata() {
    use dp_container_chain_genesis_data::TokenMetadata;
    
    let properties = Properties {
        token_metadata: TokenMetadata {
            token_symbol: b"TEST".to_vec().try_into().unwrap(),
            ss58_format: 42,
            token_decimals: 18,
        },
        is_ethereum: false,
    };
    
    assert_eq!(properties.token_metadata.token_decimals, 18);
    assert_eq!(properties.token_metadata.ss58_format, 42);
    
    // Test encoding/decoding of properties
    let encoded = properties.encode();
    let decoded = Properties::decode(&mut &encoded[..]).unwrap();
    
    assert_eq!(decoded.token_metadata.token_decimals, 18);
    assert_eq!(decoded.token_metadata.ss58_format, 42);
    assert_eq!(decoded.is_ethereum, false);
}
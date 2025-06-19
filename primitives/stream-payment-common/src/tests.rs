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

//! Unit tests for stream payment common primitives

use {
    crate::{AssetId, TimeUnit},
    parity_scale_codec::{Decode, Encode},
};

// Tests for AssetId
#[test]
fn test_asset_id_encoding() {
    let asset = AssetId::Native;
    let encoded = asset.encode();
    let decoded = AssetId::decode(&mut &encoded[..]).unwrap();
    assert_eq!(asset, decoded);
}

#[test]
fn test_asset_id_native_variant() {
    // Test that Native is the only variant and encodes as expected
    let native = AssetId::Native;

    // Test encoding size
    assert!(native.encoded_size() > 0);

    // Test round-trip encoding
    let encoded = native.encode();
    let decoded = AssetId::decode(&mut &encoded[..]).unwrap();
    assert_eq!(native, decoded);
}

// Tests for TimeUnit
#[test]
fn test_time_unit_encoding() {
    let units = vec![TimeUnit::BlockNumber, TimeUnit::Timestamp];

    for unit in units {
        let encoded = unit.encode();
        let decoded = TimeUnit::decode(&mut &encoded[..]).unwrap();
        assert_eq!(unit, decoded);
    }
}

#[test]
fn test_time_unit_variants() {
    // Test BlockNumber variant
    let block_number = TimeUnit::BlockNumber;
    assert!(matches!(block_number, TimeUnit::BlockNumber));

    // Test Timestamp variant
    let timestamp = TimeUnit::Timestamp;
    assert!(matches!(timestamp, TimeUnit::Timestamp));

    // Test that they encode differently
    let bn_encoded = block_number.encode();
    let ts_encoded = timestamp.encode();
    assert_ne!(bn_encoded, ts_encoded);
}

#[test]
fn test_time_unit_encoded_size() {
    // Both variants should have the same encoded size
    let block_number = TimeUnit::BlockNumber;
    let timestamp = TimeUnit::Timestamp;

    assert_eq!(block_number.encoded_size(), timestamp.encoded_size());
}

// Integration tests for AssetsManager and TimeProvider would require
// a full runtime setup with RuntimeConfigs trait implementation.
// These are better tested in the actual runtime integration tests
// where all the required pallets and configurations are available.

#[cfg(test)]
mod type_derivations {
    use super::*;
    use scale_info::TypeInfo;

    #[test]
    fn test_asset_id_type_info() {
        // Ensure AssetId implements TypeInfo (required for runtime types)
        let _type_info = AssetId::type_info();
    }

    #[test]
    fn test_time_unit_type_info() {
        // Ensure TimeUnit implements TypeInfo
        let _type_info = TimeUnit::type_info();
    }

    #[test]
    fn test_asset_id_traits() {
        // Test that AssetId has all required traits
        let asset = AssetId::Native;

        // Test Clone
        let cloned = asset.clone();
        assert_eq!(asset, cloned);

        // Test Copy (implicitly tested by not moving)
        let _copy1 = asset;
        let _copy2 = asset;

        // Test Debug
        let _debug_str = format!("{:?}", asset);

        // Test PartialEq and Eq (already tested above)
    }

    #[test]
    fn test_time_unit_traits() {
        // Test that TimeUnit has all required traits
        let unit = TimeUnit::BlockNumber;

        // Test Clone
        let cloned = unit.clone();
        assert_eq!(unit, cloned);

        // Test Copy
        let _copy1 = unit;
        let _copy2 = unit;

        // Test Debug
        let _debug_str = format!("{:?}", unit);
    }

    #[test]
    fn test_max_encoded_len() {
        use parity_scale_codec::MaxEncodedLen;

        // Test that AssetId implements MaxEncodedLen
        let _max_len = AssetId::max_encoded_len();

        // Test that TimeUnit implements MaxEncodedLen
        let _max_len = TimeUnit::max_encoded_len();
    }
}

// Note: Testing AssetsManager and TimeProvider implementations requires
// a full runtime with:
// - frame_system with AccountId = AccountId32 and BlockNumber = u32
// - pallet_balances with Balance = u128 and proper HoldReason configuration
// - pallet_timestamp with Moment = u64
// - All the trait bounds satisfied for RuntimeConfigs
//
// These components are tested through integration tests in the actual
// runtime implementations rather than in unit tests.

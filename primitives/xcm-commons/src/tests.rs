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

//! Unit tests for XCM commons

use {
    crate::{EthereumAssetReserve, NativeAssetReserve, Parse},
    frame_support::traits::{ContainsPair, Get},
    xcm::latest::prelude::*,
};

// Tests for Parse trait implementation on Location
#[test]
fn test_location_chain_part_sibling_parachain() {
    // Sibling parachain
    let location = Location::new(1, [Parachain(2000)]);
    let chain_part = location.chain_part();
    assert_eq!(chain_part, Some(Location::new(1, [Parachain(2000)])));
}

#[test]
fn test_location_chain_part_parent() {
    // Parent relay chain
    let location = Location::new(1, []);
    let chain_part = location.chain_part();
    assert_eq!(chain_part, Some(Location::parent()));
}

#[test]
fn test_location_chain_part_child_parachain() {
    // Child parachain
    let location = Location::new(0, [Parachain(3000)]);
    let chain_part = location.chain_part();
    assert_eq!(chain_part, Some(Location::new(0, [Parachain(3000)])));
}

#[test]
fn test_location_chain_part_none() {
    // No chain part - local account
    let location = Location::new(
        0,
        [AccountId32 {
            id: [0u8; 32],
            network: None,
        }],
    );
    let chain_part = location.chain_part();
    assert_eq!(chain_part, None);

    // No chain part - too many parents
    let location = Location::new(2, []);
    let chain_part = location.chain_part();
    assert_eq!(chain_part, None);
}

#[test]
fn test_location_chain_part_parent_with_interior() {
    // Parent with interior that is not parachain
    let location = Location::new(1, [GeneralIndex(42)]);
    let chain_part = location.chain_part();
    assert_eq!(chain_part, Some(Location::parent()));
}

// Tests for NativeAssetReserve
#[test]
fn test_native_asset_reserve_local_asset() {
    // Test local native asset
    let asset = Asset {
        id: AssetId(Location::here()),
        fun: Fungible(1000),
    };
    let origin = Location::here();

    assert!(NativeAssetReserve::contains(&asset, &origin));
}

#[test]
fn test_native_asset_reserve_parent_asset() {
    // Test parent asset from parent origin
    let asset = Asset {
        id: AssetId(Location::parent()),
        fun: Fungible(1000),
    };
    let origin = Location::parent();

    assert!(NativeAssetReserve::contains(&asset, &origin));
}

#[test]
fn test_native_asset_reserve_sibling_asset() {
    // Test sibling parachain asset
    let asset = Asset {
        id: AssetId(Location::new(1, [Parachain(2000)])),
        fun: Fungible(1000),
    };
    let origin = Location::new(1, [Parachain(2000)]);

    assert!(NativeAssetReserve::contains(&asset, &origin));
}

#[test]
fn test_native_asset_reserve_wrong_origin() {
    // Test asset from wrong origin
    let asset = Asset {
        id: AssetId(Location::new(1, [Parachain(2000)])),
        fun: Fungible(1000),
    };
    let wrong_origin = Location::new(1, [Parachain(3000)]);

    assert!(!NativeAssetReserve::contains(&asset, &wrong_origin));
}

#[test]
fn test_native_asset_reserve_complex_location() {
    // Test local asset with interior
    let asset = Asset {
        id: AssetId(Location::new(0, [GeneralIndex(42)])),
        fun: Fungible(1000),
    };
    let origin = Location::here();

    assert!(NativeAssetReserve::contains(&asset, &origin));
}

// Mock parameters for EthereumAssetReserve tests
struct MockEthereumLocation;
impl Get<Location> for MockEthereumLocation {
    fn get() -> Location {
        Location::new(1, [GlobalConsensus(Ethereum { chain_id: 1 })])
    }
}

struct MockEthereumNetwork;
impl Get<NetworkId> for MockEthereumNetwork {
    fn get() -> NetworkId {
        Ethereum { chain_id: 1 }
    }
}

type TestEthereumAssetReserve = EthereumAssetReserve<MockEthereumLocation, MockEthereumNetwork>;

#[test]
fn test_ethereum_asset_reserve_valid_eth_asset() {
    // Valid Ethereum asset from Ethereum location
    let asset = Asset {
        id: AssetId(Location::new(
            1,
            [GlobalConsensus(Ethereum { chain_id: 1 })],
        )),
        fun: Fungible(1000),
    };
    let origin = MockEthereumLocation::get();

    assert!(TestEthereumAssetReserve::contains(&asset, &origin));
}

#[test]
fn test_ethereum_asset_reserve_wrong_origin() {
    // Ethereum asset from wrong origin
    let asset = Asset {
        id: AssetId(Location::new(
            1,
            [GlobalConsensus(Ethereum { chain_id: 1 })],
        )),
        fun: Fungible(1000),
    };
    let wrong_origin = Location::new(1, [Parachain(2000)]);

    assert!(!TestEthereumAssetReserve::contains(&asset, &wrong_origin));
}

#[test]
fn test_ethereum_asset_reserve_wrong_network() {
    // Asset from different network
    let asset = Asset {
        id: AssetId(Location::new(
            1,
            [GlobalConsensus(Ethereum { chain_id: 2 })],
        )),
        fun: Fungible(1000),
    };
    let origin = MockEthereumLocation::get();

    assert!(!TestEthereumAssetReserve::contains(&asset, &origin));
}

#[test]
fn test_ethereum_asset_reserve_non_ethereum_asset() {
    // Non-Ethereum asset
    let asset = Asset {
        id: AssetId(Location::new(1, [Parachain(2000)])),
        fun: Fungible(1000),
    };
    let origin = MockEthereumLocation::get();

    assert!(!TestEthereumAssetReserve::contains(&asset, &origin));
}

#[test]
fn test_ethereum_asset_reserve_wrong_parents() {
    // Asset with wrong number of parents
    let asset = Asset {
        id: AssetId(Location::new(
            0,
            [GlobalConsensus(Ethereum { chain_id: 1 })],
        )),
        fun: Fungible(1000),
    };
    let origin = MockEthereumLocation::get();

    assert!(!TestEthereumAssetReserve::contains(&asset, &origin));
}

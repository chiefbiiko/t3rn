// Copyright (C) 2020 - 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// construct_runtime requires this
#![allow(clippy::from_over_into)]

use std::vec;

use crate::mmr::MmrLeafVersion;
use frame_support::{
    construct_runtime, parameter_types, sp_io::TestExternalities, BasicExternalities,
};
use sp_core::{Hasher, H256};
use sp_runtime::{
    app_crypto::ecdsa::Public,
    impl_opaque_keys,
    testing::Header,
    traits::{BlakeTwo256, ConvertInto, IdentityLookup, Keccak256, OpaqueKeys},
    Perbill,
};

use crate as pallet_beefy_mmr;

pub use beefy_primitives::{crypto::AuthorityId as BeefyId, ConsensusLog, BEEFY_ENGINE_ID};

impl_opaque_keys! {
    pub struct MockSessionKeys {
        pub dummy: pallet_beefy::Pallet<Test>,
    }
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        Mmr: pallet_mmr::{Pallet, Storage},
        Beefy: pallet_beefy::{Pallet, Config<T>, Storage},
        BeefyMmr: pallet_beefy_mmr::{Pallet, Storage},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Call = Call;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
}

impl pallet_session::Config for Test {
    type Event = Event;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = MockSessionManager;
    type SessionHandler = <MockSessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = MockSessionKeys;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type WeightInfo = ();
}

pub type MmrLeaf = crate::mmr::MmrLeaf<
    <Test as frame_system::Config>::BlockNumber,
    <Test as frame_system::Config>::Hash,
    <Test as pallet_mmr::Config>::Hash,
>;

impl pallet_mmr::Config for Test {
    const INDEXING_PREFIX: &'static [u8] = b"mmr";

    type Hashing = Keccak256;

    type Hash = <Keccak256 as Hasher>::Out;

    type LeafData = BeefyMmr;

    type OnNewRoot = pallet_beefy_mmr::DepositBeefyDigest<Test>;

    type WeightInfo = ();
}

impl pallet_beefy::Config for Test {
    type BeefyId = BeefyId;
}

parameter_types! {
    pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(1, 5);
}

impl pallet_beefy_mmr::Config for Test {
    type LeafVersion = LeafVersion;

    type BeefyAuthorityToMerkleLeaf = pallet_beefy_mmr::BeefyEcdsaToEthereum;

    type ParachainHeads = DummyParaHeads;
}

pub struct DummyParaHeads;
impl pallet_beefy_mmr::ParachainHeadsProvider for DummyParaHeads {
    fn parachain_heads() -> Vec<(pallet_beefy_mmr::ParaId, pallet_beefy_mmr::ParaHead)> {
        vec![(15, vec![1, 2, 3]), (5, vec![4, 5, 6])]
    }
}

pub struct MockSessionManager;
impl pallet_session::SessionManager<u64> for MockSessionManager {
    fn end_session(_: sp_staking::SessionIndex) {}
    fn start_session(_: sp_staking::SessionIndex) {}
    fn new_session(idx: sp_staking::SessionIndex) -> Option<Vec<u64>> {
        if idx == 0 || idx == 1 {
            Some(vec![1, 2])
        } else if idx == 2 {
            Some(vec![3, 4])
        } else {
            None
        }
    }
}

// Note, that we can't use `UintAuthorityId` here. Reason is that the implementation
// of `to_public_key()` assumes, that a public key is 32 bytes long. This is true for
// ed25519 and sr25519 but *not* for ecdsa. An ecdsa public key is 33 bytes.
pub fn mock_beefy_id(id: u8) -> BeefyId {
    let buf: [u8; 33] = [id; 33];
    let pk = Public::from_raw(buf);
    BeefyId::from(pk)
}

pub fn mock_authorities(vec: Vec<u8>) -> Vec<(u64, BeefyId)> {
    vec.into_iter()
        .map(|id| ((id as u64), mock_beefy_id(id)))
        .collect()
}

pub fn new_test_ext(ids: Vec<u8>) -> TestExternalities {
    new_test_ext_raw_authorities(mock_authorities(ids))
}

pub fn new_test_ext_raw_authorities(authorities: Vec<(u64, BeefyId)>) -> TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let session_keys: Vec<_> = authorities
        .iter()
        .enumerate()
        .map(|(_, id)| {
            (
                id.0 as u64,
                id.0 as u64,
                MockSessionKeys {
                    dummy: id.1.clone(),
                },
            )
        })
        .collect();

    BasicExternalities::execute_with_storage(&mut t, || {
        for (ref id, ..) in &session_keys {
            frame_system::Pallet::<Test>::inc_providers(id);
        }
    });

    pallet_session::GenesisConfig::<Test> { keys: session_keys }
        .assimilate_storage(&mut t)
        .unwrap();

    t.into()
}

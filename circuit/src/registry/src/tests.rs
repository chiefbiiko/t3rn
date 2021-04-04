#![allow(unused_must_use)]

use crate::{
    mock::*, ContractRegistry, Event as RegistryEvent, RegistryContract,
};
use frame_support::{
    assert_err, assert_ok, assert_storage_noop, StorageDoubleMap,
};
use frame_system::{EventRecord, Phase};
use sp_runtime::traits::BadOrigin;

// NOTE
// Using `run_to_block(2)` cos block#1 never includes events.
// Annotation #![allow(unused_must_use)] cos `assert_storage_noop` complains.

#[test]
fn it_stores_a_contract_in_the_registry() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        assert_ok!(Registry::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            RegistryContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        assert!(<ContractRegistry<Test>>::contains_key(
            REQUESTER,
            contract_name()
        ));

        assert_eq!(
            System::events(),
            vec![EventRecord {
                phase: Phase::Initialization,
                event: Event::pallet_registry(
                    RegistryEvent::<Test>::ContractStored(
                        REQUESTER,
                        contract_name(),
                    )
                ),
                topics: vec![],
            }]
        );
    });
}

#[test]
fn it_stores_idempotent() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        assert_ok!(Registry::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            RegistryContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        let dispatch = Registry::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            RegistryContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        );

        assert_ok!(dispatch);

        assert_storage_noop!(dispatch);

        assert_eq!(
            System::events(),
            vec![EventRecord {
                phase: Phase::Initialization,
                event: Event::pallet_registry(
                    RegistryEvent::<Test>::ContractStored(
                        REQUESTER,
                        contract_name(),
                    )
                ),
                topics: vec![],
            }]
        );
    });
}

#[test]
fn it_removes_a_contract_from_the_registry() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        assert_ok!(Registry::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            RegistryContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        assert_ok!(Registry::purge_contract(
            Origin::root(),
            REQUESTER,
            contract_name()
        ));

        assert!(!<ContractRegistry<Test>>::contains_key(
            REQUESTER,
            contract_name()
        ));

        assert_eq!(
            System::events(),
            vec![
                EventRecord {
                    phase: Phase::Initialization,
                    event: Event::pallet_registry(
                        RegistryEvent::<Test>::ContractStored(
                            REQUESTER,
                            contract_name(),
                        )
                    ),
                    topics: vec![],
                },
                EventRecord {
                    phase: Phase::Initialization,
                    event: Event::pallet_registry(
                        RegistryEvent::<Test>::ContractPurged(
                            REQUESTER,
                            contract_name(),
                        )
                    ),
                    topics: vec![],
                },
            ]
        );
    });
}

#[test]
fn it_removes_idempotent() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        assert_ok!(Registry::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            RegistryContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        assert_ok!(Registry::purge_contract(
            Origin::root(),
            REQUESTER,
            contract_name()
        ));

        let dispatch = Registry::purge_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
        );

        assert_ok!(dispatch);

        assert_storage_noop!(dispatch);

        assert_eq!(
            System::events(),
            vec![
                EventRecord {
                    phase: Phase::Initialization,
                    event: Event::pallet_registry(
                        RegistryEvent::<Test>::ContractStored(
                            REQUESTER,
                            contract_name(),
                        )
                    ),
                    topics: vec![],
                },
                EventRecord {
                    phase: Phase::Initialization,
                    event: Event::pallet_registry(
                        RegistryEvent::<Test>::ContractPurged(
                            REQUESTER,
                            contract_name(),
                        )
                    ),
                    topics: vec![],
                },
            ]
        );
    });
}

#[test]
fn it_stores_contracts_separately_per_requester() {
    assert_ne!(
        <ContractRegistry<Test>>::hashed_key_for(REQUESTER, contract_name()),
        <ContractRegistry<Test>>::hashed_key_for(
            ANOTHER_REQUESTER,
            contract_name()
        )
    );
}

#[test]
fn it_fails_for_non_root_origins() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        assert_err!(
            Registry::store_contract(
                Origin::signed(419),
                REQUESTER,
                contract_name(),
                RegistryContract {
                    code_txt: code_txt(),
                    bytes: bytes(),
                    abi: None,
                },
            ),
            BadOrigin
        );

        assert_err!(
            Registry::store_contract(
                Origin::none(),
                REQUESTER,
                contract_name(),
                RegistryContract {
                    code_txt: code_txt(),
                    bytes: bytes(),
                    abi: None,
                },
            ),
            BadOrigin
        );

        assert_err!(
            Registry::purge_contract(
                Origin::signed(419),
                REQUESTER,
                contract_name(),
            ),
            BadOrigin
        );

        assert_err!(
            Registry::purge_contract(
                Origin::none(),
                REQUESTER,
                contract_name(),
            ),
            BadOrigin
        );

        assert_eq!(System::events(), vec![]);
    });
}

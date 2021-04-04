use crate::{mock::*, ComposableContract, Error, Event as RegistryEvent, Registry};
use frame_support::{assert_ok, assert_storage_noop, StorageDoubleMap};
use frame_system::{EventRecord, Phase};

#[test]
fn it_inserts_a_contract_into_the_registry() {
    new_test_ext().execute_with(|| {
        // Fast forwarding to block#2 because block#1 does not include events.
        run_to_block(2);

        assert_ok!(ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            ComposableContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        assert!(<Registry<Test>>::contains_key(REQUESTER, contract_name()));

        assert_eq!(
            System::events(),
            vec![EventRecord {
                phase: Phase::Initialization,
                event: Event::pallet_contract_registry(RegistryEvent::<Test>::ContractStored(
                    REQUESTER,
                    contract_name(),
                )),
                topics: vec![],
            }]
        );
    });
}

#[test]
fn it_inserts_idempotent() {
    new_test_ext().execute_with(|| {
        // Fast forwarding to block#2 because block#1 does not include events.
        run_to_block(2);

        assert_ok!(ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            ComposableContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        assert_storage_noop!(ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            ComposableContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        assert_eq!(
            System::events(),
            vec![EventRecord {
                phase: Phase::Initialization,
                event: Event::pallet_contract_registry(RegistryEvent::<Test>::ContractStored(
                    REQUESTER,
                    contract_name(),
                )),
                topics: vec![],
            }]
        );
    });
}

#[test]
fn it_removes_a_contract_from_the_registry() {
    new_test_ext().execute_with(|| {
        // Fast forwarding to block#2 because block#1 does not include events.
        run_to_block(2);

        assert_ok!(ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            ComposableContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        assert_ok!(ContractRegistryModule::purge_contract(
            Origin::root(),
            REQUESTER,
            contract_name()
        ));

        assert!(!<Registry<Test>>::contains_key(REQUESTER, contract_name()));

        assert_eq!(
            System::events(),
            vec![
                EventRecord {
                    phase: Phase::Initialization,
                    event: Event::pallet_contract_registry(RegistryEvent::<Test>::ContractStored(
                        REQUESTER,
                        contract_name(),
                    )),
                    topics: vec![],
                },
                EventRecord {
                    phase: Phase::Initialization,
                    event: Event::pallet_contract_registry(RegistryEvent::<Test>::ContractPurged(
                        REQUESTER,
                        contract_name(),
                    )),
                    topics: vec![],
                },
            ]
        );
    });
}

#[test]
fn it_removes_idempotent() {
    new_test_ext().execute_with(|| {
        // Fast forwarding to block#2 because block#1 does not include events.
        run_to_block(2);

        assert_ok!(ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            ComposableContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        ));

        assert_ok!(ContractRegistryModule::purge_contract(
            Origin::root(),
            REQUESTER,
            contract_name()
        ));

        assert_storage_noop!(ContractRegistryModule::purge_contract(
            Origin::root(),
            REQUESTER,
            contract_name()
        ));

        assert_eq!(
            System::events(),
            vec![
                EventRecord {
                    phase: Phase::Initialization,
                    event: Event::pallet_contract_registry(RegistryEvent::<Test>::ContractStored(
                        REQUESTER,
                        contract_name(),
                    )),
                    topics: vec![],
                },
                EventRecord {
                    phase: Phase::Initialization,
                    event: Event::pallet_contract_registry(RegistryEvent::<Test>::ContractPurged(
                        REQUESTER,
                        contract_name(),
                    )),
                    topics: vec![],
                },
            ]
        );
    });
}

// #[test]
// fn it_stores_contracts_separately_per_requester() {
// generate storage keys for same contract but different requesters..
// ..and assert these keys are different
//     new_test_ext().execute_with(|| {});
// }

// #[test]
// fn store_fails_for_non_root_origins() {
//     new_test_ext().execute_with(|| {});
// }

// #[test]
// fn remove_fails_for_non_root_origins() {
//     new_test_ext().execute_with(|| {});
// }

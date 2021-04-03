use crate::{mock::*, ComposableContract, Error, Registry};
use frame_support::{assert_ok, assert_storage_noop, StorageDoubleMap};
use frame_system as system;

#[test]
fn it_inserts_a_contract_into_the_registry() {
    let name = contract_name(0);

    new_test_ext().execute_with(|| {
        let event_count_before = <system::Module<Test>>::event_count();

        let dispatch_result = ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            name.clone(),
            ComposableContract {
                code_txt: CODE_TXT.as_bytes().to_vec(),
                bytes: BYTES.to_vec(),
                abi: None,
            },
        );

        assert_ok!(dispatch_result);

        let exists = <Registry<Test>>::contains_key(REQUESTER, name.clone());

        assert!(exists);

        let event_count_after = <system::Module<Test>>::event_count();

        // FIXME: There are no events in the system pallet's storage.
        // Ideally tests should explicitely check event specifics..
        // ..not just the count.
        assert!(event_count_after == event_count_before + 1);
    });
}

#[test]
fn it_inserts_idempotent() {
    let name = contract_name(1);

    new_test_ext().execute_with(|| {
        let event_count_before = <system::Module<Test>>::event_count();

        let dispatch_result = ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            name.clone(),
            ComposableContract {
                code_txt: CODE_TXT.as_bytes().to_vec(),
                bytes: BYTES.to_vec(),
                abi: None,
            },
        );

        assert_ok!(dispatch_result);

        let dispatch_result = ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            name.clone(),
            ComposableContract {
                code_txt: CODE_TXT.as_bytes().to_vec(),
                bytes: BYTES.to_vec(),
                abi: None,
            },
        );

        assert_storage_noop!(dispatch_result);

        let event_count_after = <system::Module<Test>>::event_count();

        // NOTE: The storage is idempotent to multiple identical inserts..
        // ..but what we expect of the event store here is not idempotence..
        // ..even though nothing was written to storage, we expect an event.
        assert!(event_count_after == event_count_before + 2);
    });
}

#[test]
fn it_removes_a_contract_from_the_registry() {
    let name = contract_name(2);

    new_test_ext().execute_with(|| {
        let dispatch_result = ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            name.clone(),
            ComposableContract {
                code_txt: CODE_TXT.as_bytes().to_vec(),
                bytes: BYTES.to_vec(),
                abi: None,
            },
        );

        assert_ok!(dispatch_result);

        let dispatch_result =
            ContractRegistryModule::purge_contract(Origin::root(), REQUESTER, name.clone());

        assert_ok!(dispatch_result);

        let exists = <Registry<Test>>::contains_key(REQUESTER, name.clone());

        assert!(!exists);

        // TODO: assert we emitted a ContractPurged(requester, contract_name) event
    });
}

// #[test]
// fn it_removes_idempotent() {
//     new_test_ext().execute_with(|| {});
// }

// #[test]
// fn it_stores_contracts_separately_per_requester() {
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

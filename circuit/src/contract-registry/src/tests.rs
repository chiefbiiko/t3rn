use crate::{mock::*, ComposableContract, Error, Event as RegistryEvent, Registry};
use frame_support::{assert_ok, assert_storage_noop, StorageDoubleMap};
use frame_system::{EventRecord, Phase};

#[test]
fn it_inserts_a_contract_into_the_registry() {
    new_test_ext().execute_with(|| {
        // Fast forwarding to block#2 because block#1 does not include events.
        run_to_block(2);

        let dispatch_result = ContractRegistryModule::store_contract(
            Origin::root(),
            REQUESTER,
            contract_name(),
            ComposableContract {
                code_txt: code_txt(),
                bytes: bytes(),
                abi: None,
            },
        );

        assert_ok!(dispatch_result);

        let exists = <Registry<Test>>::contains_key(REQUESTER, contract_name());

        assert!(exists);

        let expected_events = vec![EventRecord {
            phase: Phase::Initialization,
            event: Event::pallet_contract_registry(RegistryEvent::<Test>::ContractStored(
                REQUESTER,
                contract_name(),
            )),
            topics: vec![],
        }];

        assert_eq!(System::events(), expected_events);
    });
}

// #[test]
// fn it_inserts_idempotent() {
//     new_test_ext().execute_with(|| {
//         // Fast forwarding to block#2 because block#1 does not include events.
//         run_to_block(2);

//         let name = contract_name(1);
//         let event_count_before = <system::Module<Test>>::event_count();

//         let dispatch_result = ContractRegistryModule::store_contract(
//             Origin::root(),
//             REQUESTER,
//             name.clone(),
//             ComposableContract {
//                 code_txt: CODE_TXT.as_bytes().to_vec(),
//                 bytes: BYTES.to_vec(),
//                 abi: None,
//             },
//         );

//         assert_ok!(dispatch_result);

//         let dispatch_result = ContractRegistryModule::store_contract(
//             Origin::root(),
//             REQUESTER,
//             name.clone(),
//             ComposableContract {
//                 code_txt: CODE_TXT.as_bytes().to_vec(),
//                 bytes: BYTES.to_vec(),
//                 abi: None,
//             },
//         );

//         assert_storage_noop!(dispatch_result);

//         let event_count_after = <system::Module<Test>>::event_count();

//         // NOTE: The storage is idempotent to multiple identical inserts..
//         // ..but what we expect of the event store here is not idempotence..
//         // ..even though nothing was written to storage, we expect an event.
//         assert!(event_count_after == event_count_before + 2);
//     });
// }

// #[test]
// fn it_removes_a_contract_from_the_registry() {
//     new_test_ext().execute_with(|| {
//         let name = contract_name(2);

//         let dispatch_result = ContractRegistryModule::store_contract(
//             Origin::root(),
//             REQUESTER,
//             name.clone(),
//             ComposableContract {
//                 code_txt: CODE_TXT.as_bytes().to_vec(),
//                 bytes: BYTES.to_vec(),
//                 abi: None,
//             },
//         );

//         assert_ok!(dispatch_result);

//         let dispatch_result =
//             ContractRegistryModule::purge_contract(Origin::root(), REQUESTER, name.clone());

//         assert_ok!(dispatch_result);

//         let exists = <Registry<Test>>::contains_key(REQUESTER, name.clone());

//         assert!(!exists);

//         // TODO: assert we emitted a ContractPurged(requester, contract_name) event
//     });
// }

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

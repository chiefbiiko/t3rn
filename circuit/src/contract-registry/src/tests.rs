use crate::{mock::*, ComposableContract, Error, Registry};
use frame_support::{assert_ok, assert_storage_noop, StorageDoubleMap, runtime_print};
use frame_system::{self as system, EventRecord, Phase};

#[test]
fn it_inserts_a_contract_into_the_registry() {
    new_test_ext().execute_with(|| {
        // Fast forwarding to block#2 because block#1 does not include events.
        run_to_block(2);

        // let event_count_before = <system::Module<Test>>::event_count();

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

        assert_eq!(System::events(), vec![
                        EventRecord {
                phase: Phase::Initialization,
                event: Event::pallet_contract_registry(crate::Event::ContractStored(REQUESTER, contract_name())),
                topics: vec![],
            },
            // EventRecord {
            //     phase: Phase::Initialization,
            //     event: Event::frame_system(frame_system::Event::NewAccount(ALICE.clone())),
            //     topics: vec![],
            // },
            // EventRecord {
            //     phase: Phase::Initialization,
            //     event: Event::pallet_balances(
            //         pallet_balances::Event::Endowed(ALICE, 1_000_000)
            //     ),
            //     topics: vec![],
            // },
            // EventRecord {
            //     phase: Phase::Initialization,
            //     event: Event::frame_system(frame_system::Event::NewAccount(addr.clone())),
            //     topics: vec![],
            // },
            // EventRecord {
            //     phase: Phase::Initialization,
            //     event: Event::pallet_balances(
            //         pallet_balances::Event::Endowed(addr.clone(), subsistence * 100)
            //     ),
            //     topics: vec![],
            // },
            // EventRecord {
            //     phase: Phase::Initialization,
            //     event: Event::pallet_balances(
            //         pallet_balances::Event::Transfer(ALICE, addr.clone(), subsistence * 100)
            //     ),
            //     topics: vec![],
            // },
            // EventRecord {
            //     phase: Phase::Initialization,
            //     event: Event::pallet_contracts(crate::Event::CodeStored(code_hash.into())),
            //     topics: vec![],
            // },
            // EventRecord {
            //     phase: Phase::Initialization,
            //     event: Event::pallet_contracts(
            //         crate::Event::ContractEmitted(addr.clone(), vec![1, 2, 3, 4])
            //     ),
            //     topics: vec![],
            // },
            // EventRecord {
            //     phase: Phase::Initialization,
            //     event: Event::pallet_contracts(crate::Event::Instantiated(ALICE, addr.clone())),
            //     topics: vec![],
            // },
        ]);

        // let event_count_after = <system::Module<Test>>::event_count();

        // let events = <system::Module<Test>>::events();

        // let event = events[0].event.clone();

        // assert_eq!(event, ContractRegistryModule::ContractStored(REQUESTER, contract_name()));
        // assert_eq!(event, <ContractStored<Test>>(REQUESTER, contract_name()));
        // runtime_print!("event {:?}", event);

        // FIXME: Ideally tests should explicitely check event specifics..
        // ..not just the count.
        // assert!(event_count_after == event_count_before + 1);
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

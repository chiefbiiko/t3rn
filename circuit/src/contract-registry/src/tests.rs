use crate::{mock::*, ComposableContract, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn onchain_registry_insert_contract() {
    new_test_ext().execute_with(|| {
        let dispatch_result = ContractRegistryModule::insert_contract(
            Origin::root(),
            REQUESTER,
            contract_name(0),
            ComposableContract {
                code_txt: CODE_TXT.as_bytes().to_vec(),
                bytes: BYTES.to_vec(),
                abi: None,
            },
        );

        assert_ok!(dispatch_result);

        // TODO: assert registry contains contract

        // TODO: assert RawEvent::ComposableContractStored(requester, contract_name)

    });
}

#[test]
fn onchain_registry_double_insert_contract() {
    new_test_ext().execute_with(|| {
        let dispatch_result = ContractRegistryModule::insert_contract(
            Origin::root(),
            REQUESTER,
            contract_name(0),
            ComposableContract {
                code_txt: CODE_TXT.as_bytes().to_vec(),
                bytes: BYTES.to_vec(),
                abi: None,
            },
        );

        assert_ok!(dispatch_result);

        let dispatch_result = ContractRegistryModule::insert_contract(
            Origin::root(),
            REQUESTER,
            contract_name(0),
            ComposableContract {
                code_txt: CODE_TXT.as_bytes().to_vec(),
                bytes: BYTES.to_vec(),
                abi: None,
            },
        );

        assert_not_ok!(dispatch_result);
    });
}

#[test]
fn onchain_registry_remove_existing_contract() {
    new_test_ext().execute_with(|| {
        let dispatch_result = ContractRegistryModule::insert_contract(
            Origin::root(),
            REQUESTER,
            contract_name(1),
            ComposableContract {
                code_txt: CODE_TXT.as_bytes().to_vec(),
                bytes: BYTES.to_vec(),
                abi: None,
            },
        );

        assert_ok!(dispatch_result);

        let dispatch_result = ContractRegistryModule::remove_contract(
            Origin::root(),
            REQUESTER,
            contract_name(1),
        );

        assert_ok!(dispatch_result);

        // TODO: assert registry NO LONGER contains contract

        // TODO: assert RawEvent::ComposableContractPurged(requester, contract_name)

    });
}

fn onchain_registry_remove_noop_non_existing_contract() {
    new_test_ext().execute_with(|| {
        let dispatch_result = ContractRegistryModule::remove_contract(
            Origin::root(),
            REQUESTER,
            contract_name(99),
        );

        assert_ok!(dispatch_result);

        // TODO: assert RawEvent::ComposableContractPurged(requester, contract_name)
        
    });
}

// #[test]
// fn correct_error_for_none_value() {
//     new_test_ext().execute_with(|| {
//         // Ensure the expected error is thrown when no value is present.
//         assert_noop!(
//             ContractRegistry::cause_error(Origin::signed(1)),
//             Error::<Test>::NoneValue
//         );
//     });
// }

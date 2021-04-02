#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get};
use frame_system;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    type Error: From<Error<Self>> + Into<<Self as frame_system::Config>::Error>;
}

#[derive(PartialEq, Eq, Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct ComposableContract {
    code_txt: Vec<u8>,
    bytes: Vec<u8>,
    abi: Option<Vec<u8>>,
}

decl_storage! {
    trait Store for Module<T: Config> as ContractRegistry {
        // https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
        // ( requester: AccountId, contract_name: Vec<u8> ) -> ComposableContract
        Registry get(fn registry):
          double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) Vec<u8> => Option<ComposableContract>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
    {
        // params [who, contract_name]
        ComposableContractStored(AccountId, Vec<u8>),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// There is no contract for the given requester and contract name.
        NoSuchContract,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;
    }
}

// https://stackoverflow.com/questions/56902167/in-substrate-is-there-a-way-to-use-storage-and-functions-from-one-custom-module?rq=1
// TODO: get rid of anyhow
impl<T: Config> Module<T> {
    pub fn get_contract(
        requester: T::AccountId,
        contract_name: Vec<u8>,
    ) -> Result<ComposableContract, T::Error> {
        if let Some(contract) = <Registry<T>>::get(requester, contract_name) {
            Ok(contract)
        } else {
            Err(Error::<T>::NoSuchContract)
        }
    }
    // pub fn put_contract(
    //     requester: T::AccountId,
    //     contract_name: Vec<u8>,
    //     contract: ComposableContract,
    // ) -> Result<(), <Module<T> as Config>::Error> {
    //     panic!("not implemented");
    // }
}

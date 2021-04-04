#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, fail};
use frame_system::ensure_root;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

/// A preliminary representation of a contract in the onchain registry.
#[derive(PartialEq, Eq, Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct ComposableContract {
    code_txt: Vec<u8>,
    bytes: Vec<u8>,
    abi: Option<Vec<u8>>,
}

decl_storage! {
    trait Store for Module<T: Config> as ContractRegistry {
        /// ( requester, contract_name ) -> Option<ComposableContract>
        Registry get(fn registry):
          double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) Vec<u8> => Option<ComposableContract>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
    {
        // Event parameters [requester, contract_name]
        ContractStored(AccountId, Vec<u8>),
        // Event parameters [requester, contract_name]
        ContractPurged(AccountId, Vec<u8>),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {}
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        /// Inserts a contract to the on-chain registry. Root only access.
        /// TODO weight
        #[weight = 10_419]
        pub fn store_contract(origin, requester: T::AccountId, contract_name: Vec<u8>, contract:ComposableContract) -> dispatch::DispatchResult {
            ensure_root(origin)?;

            if ! <Registry<T>>::contains_key(&requester, &contract_name) {
                <Registry<T>>::insert(&requester, &contract_name, contract);

                Self::deposit_event(Event::<T>::ContractStored(requester, contract_name));
            }

            Ok(())
        }

        /// Removes a contract from the on-chain registry. Root only access.
        /// TODO weight
        #[weight = 10_419]
        pub fn purge_contract(origin, requester: T::AccountId, contract_name: Vec<u8>) -> dispatch::DispatchResult {
            ensure_root(origin)?;

            if <Registry<T>>::contains_key(&requester, &contract_name) {
                <Registry<T>>::remove(&requester, &contract_name);

                Self::deposit_event(RawEvent::ContractPurged(requester, contract_name));
            }

            Ok(())
        }
    }
}

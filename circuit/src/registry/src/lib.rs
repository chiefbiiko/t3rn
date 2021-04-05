#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch,
};
use frame_system::ensure_root;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

/// A preliminary representation of a contract in the onchain registry.
#[derive(PartialEq, Eq, Encode, Decode, Default, Clone, Debug)]
pub struct RegistryContract {
    code_txt: Vec<u8>,
    bytes: Vec<u8>,
    abi: Option<Vec<u8>>,
}

decl_storage! {
    trait Store for Module<T: Config> as ContractRegistryModule {
        /// ( requester, contract_name ) -> Option<RegistryContract>
        ContractRegistry get(fn contract):
            double_map
                hasher(blake2_128_concat) T::AccountId,
                hasher(blake2_128_concat) Vec<u8>
                    => Option<RegistryContract>;
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
    pub enum Error for Module<T: Config> {
        KeyDoesNotExist,
        KeyAlreadyExists,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        /// Inserts a contract into the onchain registry. Root only access.
        /// TODO weight
        #[weight = 10_419]
        pub fn store_contract(
            origin,
            requester: T::AccountId,
            contract_name: Vec<u8>,
            contract: RegistryContract
        ) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            if <ContractRegistry<T>>::contains_key(
                &requester,
                &contract_name
            ) {
                Err(Error::<T>::KeyAlreadyExists)?
            } else {
                <ContractRegistry<T>>::insert(
                    &requester,
                    &contract_name,
                    contract
                );
                Self::deposit_event(
                    Event::<T>::ContractStored(requester, contract_name)
                );
                Ok(())
            }
        }

        /// Removes a contract from the onchain registry. Root only access.
        /// TODO weight
        #[weight = 10_419]
        pub fn purge_contract(
            origin,
            requester: T::AccountId,
            contract_name: Vec<u8>
        ) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            if !<ContractRegistry<T>>::contains_key(
                &requester,
                &contract_name
            ) {
                Err(Error::<T>::KeyDoesNotExist)?
            } else {
                <ContractRegistry<T>>::remove(&requester, &contract_name);
                Self::deposit_event(
                    Event::<T>::ContractPurged(requester, contract_name)
                );
                Ok(())
            }
        }
    }
}

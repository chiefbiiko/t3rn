// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::fake_storage::{AliveContractInfo, Storage};
use crate::simple_schedule_v2::Schedule;
use crate::{gas::GasMeter, Error, Event};

use crate::*;

use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    storage::{with_transaction, TransactionOutcome},
    traits::{Currency, ExistenceRequirement, Get, Randomness, Time},
    weights::Weight,
};

use smallvec::{Array, SmallVec};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::{
    traits::{Convert, Saturating},
    Perbill,
};

use t3rn_primitives::{transfers::BalanceOf, EscrowTrait};

use sp_std::{marker::PhantomData, mem};

pub type AccountIdOf<T> = <T as system::Config>::AccountId;
pub type MomentOf<T> = <<T as EscrowTrait>::Time as Time>::Moment;

pub type SeedOf<T> = <T as system::Config>::Hash;
pub type BlockNumberOf<T> = <T as system::Config>::BlockNumber;
pub type StorageKey = [u8; 32];
pub type ExecResult = Result<ExecReturnValue, ExecError>;

/// A type that represents a topic of an event. At the moment a hash is used.
pub type TopicOf<T> = <T as system::Config>::Hash;

/// Origin of the error.
///
/// Call or instantiate both called into other contracts and pass through errors happening
/// in those to the caller. This enum is for the caller to distinguish whether the error
/// happened during the execution of the callee or in the current execution context.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum ErrorOrigin {
    /// Caller error origin.
    ///
    /// The error happened in the current exeuction context rather than in the one
    /// of the contract that is called into.
    Caller,
    /// The error happened during execution of the called contract.
    Callee,
}

/// Error returned by contract exection.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct ExecError {
    /// The reason why the execution failed.
    pub error: DispatchError,
    /// Origin of the error.
    pub origin: ErrorOrigin,
}

impl<T: Into<DispatchError>> From<T> for ExecError {
    fn from(error: T) -> Self {
        Self {
            error: error.into(),
            origin: ErrorOrigin::Caller,
        }
    }
}

/// Information needed for rent calculations that can be requested by a contract.
#[derive(codec::Encode)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct RentParams<T: VersatileWasm> {
    /// The total balance of the contract. Includes the balance transferred from the caller.
    total_balance: BalanceOf<T>,
    /// The free balance of the contract. Includes the balance transferred from the caller.
    free_balance: BalanceOf<T>,
    /// See crate [`VVMPallet::subsistence_threshold()`].
    subsistence_threshold: BalanceOf<T>,
    /// See crate [`Config::DepositPerContract`].
    deposit_per_contract: BalanceOf<T>,
    /// See crate [`Config::DepositPerStorageByte`].
    deposit_per_storage_byte: BalanceOf<T>,
    /// See crate [`Config::DepositPerStorageItem`].
    deposit_per_storage_item: BalanceOf<T>,
    /// See crate [`Ext::rent_allowance()`].
    rent_allowance: BalanceOf<T>,
    /// See crate [`Config::RentFraction`].
    rent_fraction: Perbill,
    /// See crate [`AliveContractInfo::storage_size`].
    storage_size: u32,
    /// See crate [`Executable::aggregate_code_len()`].
    code_size: u32,
    /// See crate [`Executable::refcount()`].
    code_refcount: u32,
    /// Reserved for backwards compatible changes to this data structure.
    _reserved: Option<()>,
}

impl<T> RentParams<T>
where
    T: VersatileWasm,
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    /// Derive new `RentParams` from the passed in data.
    ///
    /// `value` is added to the current free and total balance of the contracts' account.
    fn new<E: Executable<T>>(
        account_id: &T::AccountId,
        value: &BalanceOf<T>,
        contract: &AliveContractInfo<T>,
        executable: &E,
    ) -> Self {
        Self {
            total_balance: T::Currency::total_balance(account_id).saturating_add(*value),
            free_balance: T::Currency::free_balance(account_id).saturating_add(*value),
            subsistence_threshold: T::Currency::minimum_balance(),
            deposit_per_contract: T::Currency::minimum_balance(),
            deposit_per_storage_byte: T::Currency::minimum_balance(),
            deposit_per_storage_item: T::Currency::minimum_balance(),
            rent_allowance: contract.rent_allowance,
            rent_fraction: Perbill::from_rational(4u32, 10_000u32),
            storage_size: contract.storage_size,
            code_size: executable.aggregate_code_len(),
            code_refcount: executable.refcount(),
            _reserved: None,
        }
    }
}

/// We cannot derive `Default` because `T` does not necessarily implement `Default`.
#[cfg(test)]
impl<T: VersatileWasm> Default for RentParams<T> {
    fn default() -> Self {
        Self {
            total_balance: Default::default(),
            free_balance: Default::default(),
            subsistence_threshold: Default::default(),
            deposit_per_contract: Default::default(),
            deposit_per_storage_byte: Default::default(),
            deposit_per_storage_item: Default::default(),
            rent_allowance: Default::default(),
            rent_fraction: Default::default(),
            storage_size: Default::default(),
            code_size: Default::default(),
            code_refcount: Default::default(),
            _reserved: Default::default(),
        }
    }
}

/// An interface that provides access to the external environment in which the
/// smart-contract is executed.
///
/// This interface is specialized to an account of the executing code, so all
/// operations are implicitly performed on that account.
///
/// # Note
///
/// This trait is sealed and cannot be implemented by downstream crates.
pub trait Ext {
    type T: VersatileWasm;

    /// Call (possibly transferring some amount of funds) into the specified account.
    ///
    /// Returns the original code size of the called contract.
    ///
    /// # Return Value
    ///
    /// Result<(ExecReturnValue, CodeSize), (ExecError, CodeSize)>
    fn call(
        &mut self,
        gas_limit: Weight,
        to: AccountIdOf<Self::T>,
        value: BalanceOf<Self::T>,
        input_data: Vec<u8>,
    ) -> Result<(ExecReturnValue, u32), (ExecError, u32)>;

    /// Instantiate a contract from the given code.
    ///
    /// Returns the original code size of the called contract.
    /// The newly created account will be associated with `code`. `value` specifies the amount of value
    /// transferred from this to the newly created account (also known as endowment).
    ///
    /// # Return Value
    ///
    /// Result<(AccountId, ExecReturnValue, CodeSize), (ExecError, CodeSize)>
    fn instantiate(
        &mut self,
        gas_limit: Weight,
        code: CodeHash<Self::T>,
        value: BalanceOf<Self::T>,
        input_data: Vec<u8>,
        salt: &[u8],
    ) -> Result<(AccountIdOf<Self::T>, ExecReturnValue, u32), (ExecError, u32)>;

    /// Transfer all funds to `beneficiary` and delete the contract.
    ///
    /// Returns the original code size of the terminated contract.
    /// Since this function removes the self contract eagerly, if succeeded, no further actions should
    /// be performed on this `Ext` instance.
    ///
    /// This function will fail if the same contract is present on the contract
    /// call stack.
    ///
    /// # Return Value
    ///
    /// Result<CodeSize, (DispatchError, CodeSize)>
    fn terminate(
        &mut self,
        beneficiary: &AccountIdOf<Self::T>,
    ) -> Result<u32, (DispatchError, u32)>;

    /// Restores the given destination contract sacrificing the current one.
    ///
    /// Since this function removes the self contract eagerly, if succeeded, no further actions should
    /// be performed on this `Ext` instance.
    ///
    /// This function will fail if the same contract is present
    /// on the contract call stack.
    ///
    /// # Return Value
    ///
    /// Result<(CallerCodeSize, DestCodeSize), (DispatchError, CallerCodeSize, DestCodesize)>
    fn restore_to(
        &mut self,
        dest: AccountIdOf<Self::T>,
        code_hash: CodeHash<Self::T>,
        rent_allowance: BalanceOf<Self::T>,
        delta: Vec<StorageKey>,
    ) -> Result<(u32, u32), (DispatchError, u32, u32)>;

    /// Transfer some amount of funds into the specified account.
    fn transfer(&mut self, to: &AccountIdOf<Self::T>, value: BalanceOf<Self::T>) -> DispatchResult;

    /// Returns the storage entry of the executing account by the given `key`.
    ///
    /// Returns `None` if the `key` wasn't previously set by `set_storage` or
    /// was deleted.
    fn get_storage(&mut self, key: &StorageKey) -> Option<Vec<u8>>;

    /// Sets the storage entry by the given key to the specified value. If `value` is `None` then
    /// the storage entry is deleted.
    fn set_storage(&mut self, key: StorageKey, value: Option<Vec<u8>>) -> DispatchResult;

    /// Returns a reference to the account id of the caller.
    fn caller(&self) -> &AccountIdOf<Self::T>;

    /// Returns a reference to the account id of the current contract.
    fn address(&self) -> &AccountIdOf<Self::T>;

    /// Returns the balance of the current contract.
    ///
    /// The `value_transferred` is already added.
    fn balance(&self) -> BalanceOf<Self::T>;

    /// Returns the value transferred along with this call or as endowment.
    fn value_transferred(&self) -> BalanceOf<Self::T>;

    /// Returns a reference to the timestamp of the current block
    fn now(&self) -> &MomentOf<Self::T>;

    /// Returns the minimum balance that is required for creating an account.
    fn minimum_balance(&self) -> BalanceOf<Self::T>;

    /// Returns the deposit required to create a tombstone upon contract eviction.
    fn tombstone_deposit(&self) -> BalanceOf<Self::T>;

    /// Returns a random number for the current block with the given subject.
    fn random(&self, subject: &[u8]) -> (SeedOf<Self::T>, BlockNumberOf<Self::T>);

    /// Deposit an event with the given topics.
    ///
    /// There should not be any duplicates in `topics`.
    fn deposit_event(&mut self, topics: Vec<TopicOf<Self::T>>, data: Vec<u8>);

    /// Set rent allowance of the contract
    fn set_rent_allowance(&mut self, rent_allowance: BalanceOf<Self::T>);

    /// Rent allowance of the contract
    fn rent_allowance(&mut self) -> BalanceOf<Self::T>;

    /// Returns the current block number.
    fn block_number(&self) -> BlockNumberOf<Self::T>;

    /// Returns the maximum allowed size of a storage item.
    fn max_value_size(&self) -> u32;

    /// Returns the price for the specified amount of weight.
    fn get_weight_price(&self, weight: Weight) -> BalanceOf<Self::T>;

    /// Get a reference to the schedule used by the current call.
    fn schedule(&self) -> &Schedule;

    /// Information needed for rent calculations.
    fn rent_params(&self) -> &RentParams<Self::T>;

    /// Get a mutable reference to the nested gas meter.
    fn gas_meter(&mut self) -> &mut GasMeter<Self::T>;

    /// Append a string to the debug buffer.
    ///
    /// It is added as-is without any additional new line.
    ///
    /// This is a no-op if debug message recording is disabled which is always the case
    /// when the code is executing on-chain.
    ///
    /// Returns `true` if debug message recording is enabled. Otherwise `false` is returned.
    fn append_debug_buffer(&mut self, msg: &str) -> bool;
}

/// Describes the different functions that can be exported by an [`Executable`].
#[derive(Clone, Copy, PartialEq)]
pub enum ExportedFunction {
    /// The constructor function which is executed on deployment of a contract.
    Constructor,
    /// The function which is executed when a contract is called.
    Call,
}

/// A trait that represents something that can be executed.
///
/// In the on-chain environment this would be represented by a wasm module. This trait exists in
/// order to be able to mock the wasm logic for testing.
pub trait Executable<T: VersatileWasm>: Sized + Clone + Copy {
    /// Load the executable from storage.
    fn from_storage(
        code_hash: CodeHash<T>,
        schedule: &Schedule,
        gas_meter: &mut GasMeter<T>,
    ) -> Result<Self, DispatchError>;

    /// Load the module from storage without re-instrumenting it.
    ///
    /// A code module is re-instrumented on-load when it was originally instrumented with
    /// an older schedule. This skips this step for cases where the code storage is
    /// queried for purposes other than execution.
    fn from_storage_noinstr(code_hash: CodeHash<T>) -> Result<Self, DispatchError>;

    /// Decrements the refcount by one and deletes the code if it drops to zero.
    fn drop_from_storage(self);

    /// Increment the refcount by one. Fails if the code does not exist on-chain.
    ///
    /// Returns the size of the original code.
    fn add_user(code_hash: CodeHash<T>) -> Result<u32, DispatchError>;

    /// Decrement the refcount by one and remove the code when it drops to zero.
    ///
    /// Returns the size of the original code.
    fn remove_user(code_hash: CodeHash<T>) -> u32;

    /// Execute the specified exported function and return the result.
    ///
    /// When the specified function is `Constructor` the executable is stored and its
    /// refcount incremented.
    ///
    /// # Note
    ///
    /// This functions expects to be executed in a storage transaction that rolls back
    /// all of its emitted storage changes.
    fn execute<E: Ext<T = T>>(
        self,
        ext: &mut E,
        function: &ExportedFunction,
        input_data: Vec<u8>,
    ) -> ExecResult;

    /// The code hash of the executable.
    fn code_hash(&self) -> &CodeHash<T>;

    /// Size of the instrumented code in bytes.
    fn code_len(&self) -> u32;

    /// Sum of instrumented and pristine code len.
    fn aggregate_code_len(&self) -> u32;

    // The number of contracts using this executable.
    fn refcount(&self) -> u32;

    /// The storage that is occupied by the instrumented executable and its pristine source.
    ///
    /// The returned size is already divided by the number of users who share the code.
    /// This is essentially `aggregate_code_len() / refcount()`.
    ///
    /// # Note
    ///
    /// This works with the current in-memory value of refcount. When calling any contract
    /// without refetching this from storage the result can be inaccurate as it might be
    /// working with a stale value. Usually this inaccuracy is tolerable.
    fn occupied_storage(&self) -> u32 {
        // We disregard the size of the struct itself as the size is completely
        // dominated by the code size.
        let len = self.aggregate_code_len();
        len.checked_div(self.refcount()).unwrap_or(len)
    }
}

/// The complete call stack of a contract execution.
///
/// The call stack is initiated by either a signed origin or one of the contract RPC calls.
/// This type implements `Ext` and by that exposes the business logic of contract execution to
/// the runtime module which interfaces with the contract (the wasm blob) itself.
pub struct Stack<'a, T: VersatileWasm, E> {
    /// The account id of a plain account that initiated the call stack.
    ///
    /// # Note
    ///
    /// Please note that it is possible that the id belongs to a contract rather than a plain
    /// account when being called through one of the contract RPCs where the client can freely
    /// choose the origin. This usually makes no sense but is still possible.
    origin: T::AccountId,
    /// The cost schedule used when charging from the gas meter.
    schedule: &'a Schedule,
    /// The gas meter where costs are charged to.
    gas_meter: &'a mut GasMeter<T>,
    /// The timestamp at the point of call stack instantiation.
    timestamp: MomentOf<T>,
    /// The block number at the time of call stack instantiation.
    block_number: T::BlockNumber,
    /// The account counter is cached here when accessed. It is written back when the call stack
    /// finishes executing.
    account_counter: Option<u64>,
    /// The actual call stack. One entry per nested contract called/instantiated.
    /// This does **not** include the [`Self::first_frame`].
    frames: SmallVec<T::CallStack>,
    /// Statically guarantee that each call stack has at least one frame.
    first_frame: Frame<T>,
    /// A text buffer used to output human readable information.
    ///
    /// All the bytes added to this field should be valid UTF-8. The buffer has no defined
    /// structure and is intended to be shown to users as-is for debugging purposes.
    debug_message: Option<&'a mut Vec<u8>>,
    /// No executable is held by the struct but influences its behaviour.
    _phantom: PhantomData<E>,
}

/// Represents one entry in the call stack.
///
/// For each nested contract call or instantiate one frame is created. It holds specific
/// information for the said call and caches the in-storage `ContractInfo` data structure.
///
/// # Note
///
/// This is an internal data structure. It is exposed to the public for the sole reason
/// of specifying [`Config::CallStack`].
pub struct Frame<T: VersatileWasm> {
    /// The account id of the executing contract.
    account_id: T::AccountId,
    /// The cached in-storage data of the contract.
    contract_info: CachedContract<T>,
    /// The amount of balance transferred by the caller as part of the call.
    value_transferred: BalanceOf<T>,
    /// Snapshotted rent information that can be copied to the contract if requested.
    rent_params: RentParams<T>,
    /// Determines whether this is a call or instantiate frame.
    entry_point: ExportedFunction,
    /// The gas meter capped to the supplied gas limit.
    nested_meter: GasMeter<T>,
}

/// Parameter passed in when creating a new `Frame`.
///
/// It determines whether the new frame is for a call or an instantiate.
pub enum FrameArgs<'a, T: VersatileWasm, E> {
    Call {
        /// The account id of the contract that is to be called.
        dest: T::AccountId,
        /// If `None` the contract info needs to be reloaded from storage.
        cached_info: Option<AliveContractInfo<T>>,
    },
    Instantiate {
        /// The contract or signed origin which instantiates the new contract.
        sender: T::AccountId,
        /// The seed that should be used to derive a new trie id for the contract.
        trie_seed: u64,
        /// The executable whose `deploy` function is run.
        executable: E,
        /// A salt used in the contract address deriviation of the new contract.
        salt: &'a [u8],
    },
}

/// Describes the different states of a contract as contained in a `Frame`.
pub enum CachedContract<T: VersatileWasm> {
    /// The cached contract is up to date with the in-storage value.
    Cached(AliveContractInfo<T>),
    /// A recursive call into the same contract did write to the contract info.
    ///
    /// In this case the cached contract is stale and needs to be reloaded from storage.
    Invalidated,
    /// The current contract executed `terminate` or `restore_to` and removed the contract.
    ///
    /// In this case a reload is neither allowed nor possible. Please note that recursive
    /// calls cannot remove a contract as this is checked and denied.
    Terminated,
}

impl<T: VersatileWasm> Frame<T> {
    /// Return the `contract_info` of the current contract.
    pub fn contract_info(&mut self) -> &mut AliveContractInfo<T> {
        self.contract_info.as_alive(&self.account_id)
    }

    /// Invalidate and return the `contract_info` of the current contract.
    pub fn invalidate(&mut self) -> AliveContractInfo<T> {
        self.contract_info.invalidate(&self.account_id)
    }

    /// Terminate and return the `contract_info` of the current contract.
    ///
    /// # Note
    ///
    /// Under no circumstances the contract is allowed to access the `contract_info` after
    /// a call to this function. This would constitute a programming error in the exec module.
    pub fn terminate(&mut self) -> AliveContractInfo<T> {
        self.contract_info.terminate(&self.account_id)
    }
}

/// Extract the contract info after loading it from storage.
///
/// This assumes that `load` was executed before calling this macro.
macro_rules! get_cached_or_panic_after_load {
    ($c:expr) => {{
        if let CachedContract::Cached(contract) = $c {
            contract
        } else {
            panic!(
                "It is impossible to remove a contract that is on the call stack;\
				See implementations of terminate and restore_to;\
				Therefore fetching a contract will never fail while using an account id
				that is currently active on the call stack;\
				qed"
            );
        }
    }};
}

impl<T: VersatileWasm> CachedContract<T> {
    /// Load the `contract_info` from storage if necessary.
    pub fn load(&mut self, _account_id: &T::AccountId) {
        unimplemented!("contracts are not supposed to be stored using VVM")
    }

    /// Return the cached contract_info as alive contract info.
    pub fn as_alive(&mut self, account_id: &T::AccountId) -> &mut AliveContractInfo<T> {
        self.load(account_id);
        get_cached_or_panic_after_load!(self)
    }

    /// Invalidate and return the contract info.
    pub fn invalidate(&mut self, account_id: &T::AccountId) -> AliveContractInfo<T> {
        self.load(account_id);
        get_cached_or_panic_after_load!(mem::replace(self, Self::Invalidated))
    }

    /// Terminate and return the contract info.
    pub fn terminate(&mut self, account_id: &T::AccountId) -> AliveContractInfo<T> {
        self.load(account_id);
        get_cached_or_panic_after_load!(mem::replace(self, Self::Terminated))
    }
}

impl<'a, T, E> Stack<'a, T, E>
where
    T: VersatileWasm,
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
    E: Executable<T>,
{
    /// Create an run a new call stack by calling into `dest`.
    ///
    /// # Note
    ///
    /// `debug_message` should only ever be set to `Some` when executing as an RPC because
    /// it adds allocations and could be abused to drive the runtime into an OOM panic.
    ///
    /// # Return Value
    ///
    /// Result<(ExecReturnValue, CodeSize), (ExecError, CodeSize)>
    pub fn run_call(
        origin: T::AccountId,
        dest: T::AccountId,
        gas_meter: &'a mut GasMeter<T>,
        schedule: &'a Schedule,
        value: BalanceOf<T>,
        input_data: Vec<u8>,
        debug_message: Option<&'a mut Vec<u8>>,
    ) -> Result<(ExecReturnValue, u32), (ExecError, u32)> {
        let (mut stack, executable) = Self::new(
            FrameArgs::Call {
                dest,
                cached_info: None,
            },
            origin,
            gas_meter,
            schedule,
            value,
            debug_message,
        )?;
        stack.run(executable, input_data)
    }

    /// Create and run a new call stack by instantiating a new contract.
    ///
    /// # Note
    ///
    /// `debug_message` should only ever be set to `Some` when executing as an RPC because
    /// it adds allocations and could be abused to drive the runtime into an OOM panic.
    ///
    /// # Return Value
    ///
    /// Result<(NewContractAccountId, ExecReturnValue), ExecError)>
    pub fn run_instantiate(
        origin: T::AccountId,
        executable: E,
        gas_meter: &'a mut GasMeter<T>,
        schedule: &'a Schedule,
        value: BalanceOf<T>,
        input_data: Vec<u8>,
        salt: &[u8],
        debug_message: Option<&'a mut Vec<u8>>,
    ) -> Result<(T::AccountId, ExecReturnValue), ExecError> {
        let (mut stack, executable) = Self::new(
            FrameArgs::Instantiate {
                sender: origin.clone(),
                trie_seed: Self::initial_trie_seed(),
                executable,
                salt,
            },
            origin,
            gas_meter,
            schedule,
            value,
            debug_message,
        )
        .map_err(|(e, _code_len)| e)?;
        let account_id = stack.top_frame().account_id.clone();
        stack
            .run(executable, input_data)
            .map(|(ret, _code_len)| (account_id, ret))
            .map_err(|(err, _code_len)| err)
    }

    /// Create a new call stack.
    fn new(
        args: FrameArgs<T, E>,
        origin: T::AccountId,
        gas_meter: &'a mut GasMeter<T>,
        schedule: &'a Schedule,
        value: BalanceOf<T>,
        debug_message: Option<&'a mut Vec<u8>>,
    ) -> Result<(Self, E), (ExecError, u32)> {
        let (first_frame, executable) = Self::new_frame(args, value, gas_meter, 0, &schedule)?;
        let stack = Self {
            origin,
            schedule,
            gas_meter,
            timestamp: T::Time::now(),
            block_number: <system::Pallet<T>>::block_number(),
            account_counter: None,
            first_frame,
            frames: Default::default(),
            debug_message,
            _phantom: Default::default(),
        };

        Ok((stack, executable))
    }

    /// Construct a new frame.
    ///
    /// This does not take `self` because when constructing the first frame `self` is
    /// not initialized, yet.
    fn new_frame(
        frame_args: FrameArgs<T, E>,
        value_transferred: BalanceOf<T>,
        gas_meter: &mut GasMeter<T>,
        gas_limit: Weight,
        schedule: &Schedule,
    ) -> Result<(Frame<T>, E), (ExecError, u32)> {
        let (account_id, contract_info, executable, entry_point) = match frame_args {
            FrameArgs::Call { dest, cached_info } => {
                let contract = if let Some(contract) = cached_info {
                    contract
                } else {
                    return Err((Error::<T>::NotCallable.into(), 0));
                };

                let executable = E::from_storage(contract.code_hash, schedule, gas_meter)
                    .map_err(|e| (e.into(), 0))?;

                // This charges the rent and denies access to a contract that is in need of
                // eviction by returning `None`. We cannot evict eagerly here because those
                // changes would be rolled back in case this contract is called by another
                // contract.
                // See: https://github.com/paritytech/substrate/issues/6439#issuecomment-648754324
                // let contract = Rent::<T, E>::charge(&dest, contract, executable.occupied_storage())
                //     .map_err(|e| (e.into(), executable.code_len()))?
                //     .ok_or((Error::<T>::NotCallable.into(), executable.code_len()))?;
                (dest, contract, executable, ExportedFunction::Call)
            }
            FrameArgs::Instantiate {
                sender: _,
                trie_seed: _,
                executable: _,
                salt: _,
            } => unimplemented!("contracts are not supposed to be stored using VVM"),
        };

        let frame = Frame {
            rent_params: RentParams::new(
                &account_id,
                &value_transferred,
                &contract_info,
                &executable,
            ),
            value_transferred,
            contract_info: CachedContract::Cached(contract_info),
            account_id,
            entry_point,
            nested_meter: gas_meter
                .nested(gas_limit)
                .map_err(|e| (e.into(), executable.code_len()))?,
        };

        Ok((frame, executable))
    }

    /// Create a subsequent nested frame.
    fn push_frame(
        &mut self,
        frame_args: FrameArgs<T, E>,
        value_transferred: BalanceOf<T>,
        gas_limit: Weight,
    ) -> Result<E, (ExecError, u32)> {
        if self.frames.len() == T::CallStack::size() {
            return Err((Error::<T>::MaxCallDepthReached.into(), 0));
        }

        // We need to make sure that changes made to the contract info are not discarded.
        // See the `in_memory_changes_not_discarded` test for more information.
        // We do not store on instantiate because we do not allow to call into a contract
        // from its own constructor.
        let _frame = self.top_frame();
        let nested_meter = &mut self
            .frames
            .last_mut()
            .unwrap_or(&mut self.first_frame)
            .nested_meter;
        let (frame, executable) = Self::new_frame(
            frame_args,
            value_transferred,
            nested_meter,
            gas_limit,
            self.schedule,
        )?;
        self.frames.push(frame);
        Ok(executable)
    }

    /// Run the current (top) frame.
    ///
    /// This can be either a call or an instantiate.
    fn run(
        &mut self,
        executable: E,
        input_data: Vec<u8>,
    ) -> Result<(ExecReturnValue, u32), (ExecError, u32)> {
        let entry_point = self.top_frame().entry_point;
        let do_transaction = || {
            // Cache the value before calling into the constructor because that
            // consumes the value. If the constructor creates additional contracts using
            // the same code hash we still charge the "1 block rent" as if they weren't
            // spawned. This is OK as overcharging is always safe.
            let _occupied_storage = executable.occupied_storage();
            let code_len = executable.code_len();

            // Every call or instantiate also optionally transferres balance.
            self.initial_transfer()
                .map_err(|e| (ExecError::from(e), 0))?;

            let code_hash = executable.code_hash().clone();

            // Call into the wasm blob.
            let output = executable
                .execute(self, &entry_point, input_data)
                .map_err(|e| {
                    (
                        ExecError {
                            error: e.error,
                            origin: ErrorOrigin::Callee,
                        },
                        code_len,
                    )
                })?;

            // Additional work needs to be performed in case of an instantiation.
            if output.is_success() && entry_point == ExportedFunction::Constructor {
                let frame = self.top_frame_mut();
                let account_id = frame.account_id.clone();

                // It is not allowed to terminate a contract inside its constructor.
                if let CachedContract::Terminated = frame.contract_info {
                    return Err((Error::<T>::TerminatedInConstructor.into(), code_len));
                }

                // Collect the rent for the first block to prevent the creation of very large
                // contracts that never intended to pay for even one block.
                // This also makes sure that it is above the subsistence threshold
                // in order to keep up the guarantuee that we always leave a tombstone behind
                // with the exception of a contract that called `seal_terminate`.
                // let contract =
                //     Rent::<T, E>::charge(&account_id, frame.invalidate(), occupied_storage)
                //         .map_err(|e| (e.into(), code_len))?
                //         .ok_or((Error::<T>::NewContractNotFunded.into(), code_len))?;

                let trie_seed = Self::initial_trie_seed();

                let trie_id = Storage::<T>::generate_trie_id(&account_id, trie_seed);
                let contract = Storage::<T>::new_contract(&account_id, trie_id, code_hash)
                    .map_err(|e| (e.into(), executable.code_len()))?;

                frame.contract_info = CachedContract::Cached(contract);

                // Deposit an instantiation event.
                deposit_event::<T>(
                    vec![],
                    Event::TempInstantiated(self.caller().clone(), account_id),
                );
            }

            Ok((output, code_len))
        };

        // All changes performed by the contract are executed under a storage transaction.
        // This allows for roll back on error. Changes to the cached contract_info are
        // comitted or rolled back when popping the frame.
        let (success, output) = with_transaction(|| {
            let output = do_transaction();
            match output {
                Ok((ref result, _)) if result.is_success() => {
                    TransactionOutcome::Commit((true, output))
                }
                _ => TransactionOutcome::Rollback((false, output)),
            }
        });
        self.pop_frame(success);
        output
    }

    /// Remove the current (top) frame from the stack.
    ///
    /// This is called after running the current frame. It commits cached values to storage
    /// and invalidates all stale references to it that might exist further down the call stack.
    fn pop_frame(&mut self, persist: bool) {
        // Revert the account counter in case of a failed instantiation.
        if !persist && self.top_frame().entry_point == ExportedFunction::Constructor {
            self.account_counter
                .as_mut()
                .map(|c| *c = c.wrapping_sub(1));
        }

        // Pop the current frame from the stack and return it in case it needs to interact
        // with duplicates that might exist on the stack.
        // A `None` means that we are returning from the `first_frame`.
        let frame = self.frames.pop();

        if let Some(frame) = frame {
            let prev = self.top_frame_mut();
            let account_id = &frame.account_id;
            prev.nested_meter.absorb_nested(frame.nested_meter);
            // Only gas counter changes are persisted in case of a failure.
            if !persist {
                return;
            }
            if let CachedContract::Cached(contract) = frame.contract_info {
                // optimization: Predecessor is the same contract.
                // We can just copy the contract into the predecessor without a storage write.
                // This is possible when there is no other contract in-between that could
                // trigger a rollback.
                if prev.account_id == *account_id {
                    prev.contract_info = CachedContract::Cached(contract);
                    return;
                }

                // Predecessor is a different contract: We persist the info and invalidate the first
                // stale cache we find. This triggers a reload from storage on next use. We skip(1)
                // because that case is already handled by the optimization above. Only the first
                // cache needs to be invalidated because that one will invalidate the next cache
                // when it is popped from the stack.
                // <ContractInfoOf<T>>::insert(account_id, ContractInfo::Alive(contract));
                if let Some(c) = self
                    .frames_mut()
                    .skip(1)
                    .find(|f| f.account_id == *account_id)
                {
                    c.contract_info = CachedContract::Invalidated;
                }
            }
        } else {
            if let Some(message) = &self.debug_message {
                log::debug!(
                    target: "runtime::contracts",
                    "Debug Message: {}",
                    core::str::from_utf8(message).unwrap_or("<Invalid UTF8>"),
                );
            }
            // Write back to the root gas meter.
            self.gas_meter
                .absorb_nested(mem::take(&mut self.first_frame.nested_meter));
            // Always do not persist in VVM
            if let Some(counter) = self.account_counter {
                <AccountCounter<T>>::set(counter);
            }

            return;
        }
    }

    /// Transfer some funds from `from` to `to`.
    ///
    /// We only allow allow for draining all funds of the sender if `allow_death` is
    /// is specified as `true`. Otherwise, any transfer that would bring the sender below the
    /// subsistence threshold (for contracts) or the existential deposit (for plain accounts)
    /// results in an error.
    fn transfer(
        sender_is_contract: bool,
        allow_death: bool,
        from: &T::AccountId,
        to: &T::AccountId,
        value: BalanceOf<T>,
    ) -> DispatchResult {
        if value == 0u32.into() {
            return Ok(());
        }

        let existence_requirement = match (allow_death, sender_is_contract) {
            (true, _) => ExistenceRequirement::AllowDeath,
            (false, true) => {
                ensure!(
                    T::Currency::total_balance(from).saturating_sub(value)
                        >= T::Currency::minimum_balance(),
                    Error::<T>::BelowSubsistenceThreshold
                );
                ExistenceRequirement::KeepAlive
            }
            (false, false) => ExistenceRequirement::KeepAlive,
        };

        T::Currency::transfer(from, to, value, existence_requirement)
            .map_err(|_| Error::<T>::TransferFailed)?;

        Ok(())
    }

    // The transfer as performed by a call or instantiate.
    fn initial_transfer(&self) -> DispatchResult {
        Self::transfer(
            self.caller_is_origin(),
            false,
            self.caller(),
            &self.top_frame().account_id,
            self.top_frame().value_transferred,
        )
    }

    /// Wether the caller is the initiator of the call stack.
    fn caller_is_origin(&self) -> bool {
        !self.frames.is_empty()
    }

    /// Reference to the current (top) frame.
    fn top_frame(&self) -> &Frame<T> {
        self.frames.last().unwrap_or(&self.first_frame)
    }

    /// Mutable reference to the current (top) frame.
    fn top_frame_mut(&mut self) -> &mut Frame<T> {
        self.frames.last_mut().unwrap_or(&mut self.first_frame)
    }

    /// Iterator over all frames.
    ///
    /// The iterator starts with the top frame and ends with the root frame.
    fn frames(&self) -> impl Iterator<Item = &Frame<T>> {
        sp_std::iter::once(&self.first_frame)
            .chain(&self.frames)
            .rev()
    }

    /// Same as `frames` but with a mutable reference as iterator item.
    pub fn frames_mut(&mut self) -> impl Iterator<Item = &mut Frame<T>> {
        sp_std::iter::once(&mut self.first_frame)
            .chain(&mut self.frames)
            .rev()
    }

    /// Returns whether the current contract is on the stack multiple times.
    pub fn is_recursive(&self) -> bool {
        let account_id = &self.top_frame().account_id;
        self.frames().skip(1).any(|f| &f.account_id == account_id)
    }

    /// Increments the cached account id and returns the value to be used for the trie_id.
    pub fn next_trie_seed(&mut self) -> u64 {
        let next = if let Some(current) = self.account_counter {
            current + 1
        } else {
            Self::initial_trie_seed()
        };
        self.account_counter = Some(next);
        next
    }

    /// The account seed to be used to instantiate the account counter cache.
    fn initial_trie_seed() -> u64 {
        <AccountCounter<T>>::get().wrapping_add(1)
    }
}

impl<'a, T, E> Ext for Stack<'a, T, E>
where
    T: VersatileWasm,
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
    E: Executable<T>,
{
    type T = T;

    fn call(
        &mut self,
        gas_limit: Weight,
        to: T::AccountId,
        value: BalanceOf<T>,
        input_data: Vec<u8>,
    ) -> Result<(ExecReturnValue, u32), (ExecError, u32)> {
        // We ignore instantiate frames in our search for a cached contract.
        // Otherwise it would be possible to recursively call a contract from its own
        // constructor: We disallow calling not fully constructed contracts.
        let cached_info = self
            .frames()
            .find(|f| f.entry_point == ExportedFunction::Call && f.account_id == to)
            .and_then(|f| match &f.contract_info {
                CachedContract::Cached(contract) => Some(contract.clone()),
                _ => None,
            });
        let executable = self.push_frame(
            FrameArgs::Call {
                dest: to,
                cached_info,
            },
            value,
            gas_limit,
        )?;
        self.run(executable, input_data)
    }

    fn instantiate(
        &mut self,
        _gas_limit: Weight,
        _code_hash: CodeHash<T>,
        _endowment: BalanceOf<T>,
        _input_data: Vec<u8>,
        _salt: &[u8],
    ) -> Result<(AccountIdOf<T>, ExecReturnValue, u32), (ExecError, u32)> {
        unimplemented!("contracts are not supposed to be stored using VVM")
    }

    fn terminate(
        &mut self,
        _beneficiary: &AccountIdOf<Self::T>,
    ) -> Result<u32, (DispatchError, u32)> {
        unimplemented!("contracts are not supposed to be stored using VVM")

        // if self.is_recursive() {
        //     return Err((Error::<T>::ReentranceDenied.into(), 0));
        // }
        // let frame = self.top_frame_mut();
        // let info = frame.terminate();
        // Storage::<T>::queue_trie_for_deletion(&info).map_err(|e| (e, 0))?;
        // <Stack<'a, T, E>>::transfer(
        //     true,
        //     true,
        //     &frame.account_id,
        //     beneficiary,
        //     T::Currency::free_balance(&frame.account_id),
        // )
        // .map_err(|e| (e, 0))?;
        // ContractInfoOf::<T>::remove(&frame.account_id);
        // let code_len = E::remove_user(info.code_hash);
        // VVMPallet::<T>::deposit_event(Event::Terminated(
        //     frame.account_id.clone(),
        //     beneficiary.clone(),
        // ));
        // Ok(code_len)
    }

    fn restore_to(
        &mut self,
        _dest: AccountIdOf<Self::T>,
        _code_hash: CodeHash<Self::T>,
        _rent_allowance: BalanceOf<Self::T>,
        _delta: Vec<StorageKey>,
    ) -> Result<(u32, u32), (DispatchError, u32, u32)> {
        unimplemented!("contracts are not supposed to be stored using VVM")
    }

    fn transfer(&mut self, to: &T::AccountId, value: BalanceOf<T>) -> DispatchResult {
        Self::transfer(true, false, &self.top_frame().account_id, to, value)
    }

    fn get_storage(&mut self, key: &StorageKey) -> Option<Vec<u8>> {
        Storage::<T>::read(&self.top_frame_mut().contract_info().trie_id, key)
    }

    fn set_storage(&mut self, key: StorageKey, value: Option<Vec<u8>>) -> DispatchResult {
        let block_number = self.block_number;
        let frame = self.top_frame_mut();
        Storage::<T>::write(block_number, frame.contract_info(), &key, value)
    }

    fn caller(&self) -> &T::AccountId {
        self.frames()
            .nth(1)
            .map(|f| &f.account_id)
            .unwrap_or(&self.origin)
    }

    fn address(&self) -> &T::AccountId {
        &self.top_frame().account_id
    }

    fn balance(&self) -> BalanceOf<T> {
        T::Currency::free_balance(&self.top_frame().account_id)
    }

    fn value_transferred(&self) -> BalanceOf<T> {
        self.top_frame().value_transferred
    }

    fn now(&self) -> &MomentOf<T> {
        &self.timestamp
    }

    fn minimum_balance(&self) -> BalanceOf<T> {
        T::Currency::minimum_balance()
    }

    fn tombstone_deposit(&self) -> BalanceOf<T> {
        <Self::T as EscrowTrait>::Currency::minimum_balance()
    }

    fn random(&self, subject: &[u8]) -> (SeedOf<T>, BlockNumberOf<T>) {
        T::Randomness::random(subject)
    }

    fn deposit_event(&mut self, topics: Vec<T::Hash>, data: Vec<u8>) {
        deposit_event::<Self::T>(
            topics,
            Event::ContractEmitted(self.top_frame().account_id.clone(), data),
        );
    }

    fn set_rent_allowance(&mut self, rent_allowance: BalanceOf<T>) {
        self.top_frame_mut().contract_info().rent_allowance = rent_allowance;
    }

    fn rent_allowance(&mut self) -> BalanceOf<T> {
        self.top_frame_mut().contract_info().rent_allowance
    }

    fn block_number(&self) -> T::BlockNumber {
        self.block_number
    }

    fn max_value_size(&self) -> u32 {
        T::Schedule::get().max_memory_pages
    }

    fn get_weight_price(&self, weight: Weight) -> BalanceOf<Self::T> {
        T::WeightPrice::convert(weight)
    }

    fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    fn rent_params(&self) -> &RentParams<Self::T> {
        &self.top_frame().rent_params
    }

    fn gas_meter(&mut self) -> &mut GasMeter<Self::T> {
        &mut self.top_frame_mut().nested_meter
    }

    fn append_debug_buffer(&mut self, msg: &str) -> bool {
        if let Some(buffer) = &mut self.debug_message {
            if !msg.is_empty() {
                buffer.extend(msg.as_bytes());
            }
            true
        } else {
            false
        }
    }
}

fn deposit_event<T: VersatileWasm>(topics: Vec<T::Hash>, event: Event<T>) {
    <system::Pallet<T>>::deposit_event_indexed(
        &*topics,
        <T as VersatileWasm>::Event::from(event).into(),
    )
}
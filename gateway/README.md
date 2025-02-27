## Gateway

t3rn comes with the standalone Gateway pallet for multi-steps transaction processing that brings a possibility of safely reverting the changes based on external of parachain factors and spread over time. 

The standalone version of the gateway brings additional phases over the regular runtime execution - commit and revert, due to which effects of the execution can be reverted and never committed to the target accounts. This is possible, as after the execution phase the changes are made only to the child storage of dedicated on that parachain Escrow Account, which acts as a regular account from the perspective of a parachain, therefore all of the changes to its storage are secured by the consensus effective in that parachain and can be relied on by already integrated services. 

### User Guide
#### Architecture overview

t3rn supports both Parachains with and without Contracts. The common features between both gateways are:
##### Escrow Balance Transfers
Secure multi-phase balance transfers.
- Single escrow transfer can be invoked [when no code is attached to the call](pallets/src/tests.rs:85L). 
- Multiple escrow transfers can be invoked when attaching an appropriate code containing the transfers from within the contract (see [transfer_return_code.wat` example](/pallets/fixtures/transfer_return_code.wat)).

Funds are secured between execution phases on `escrow_account`:
- `execution` phase moves the funds from `requester` to `escrow_account` 
- `revert` phase refunds `requester` to `escrow_account`.
- `commit` phase moves funds from `escrow_account` to `target_accounts`.

##### Escrow Generic Code Execution
Secure multi-phase balance transfers.
- Single escrow transfer can be invoked [when no code is attached to the call](pallets/src/tests.rs:85L). 
- Multiple escrow transfers can be invoked when attaching an appropriate code containing the transfers from within the contract (see [transfer_return_code.wat` example](/pallets/fixtures/transfer_return_code.wat)).

Funds are secured between execution phases on `escrow_account`:
- `execution` phase moves the funds from `requester` to `escrow_account` 
- `revert` phase refunds `requester` to `escrow_account`.
- `commit` phase moves funds from `escrow_account` to `target_accounts`.

##### Escrow Generic Code Execution & Escrow Contract Calls
`Escrow Generic Code Execution` is possible on both parachains regardless of whether they implement the contracts or not. 
Attached to the call `code` is executed in a secured by escrow account environment,which results in two effects: `deferred_storage_writes` (storage changes to affected by execution contracts) and `deferred_result` (output of the execution).

`Escrow Contract Calls` is only possible on parachains implementing contracts. Secure multi-phase calls to contracts already registered on the processing parachain (or instantiated by attaching the code). Recursive calls are also supported. Effects to all of the affected contracts will be postponed until the `commit` phase. 

Similarly to attached code execution of contract call results in twofold effects: `deferred_storage_writes` and`deferred_result`.

- `execution` executes the contract at `target_dest` with given `input_data`. All of the storage writes during the execution are collected into `deferred_writes`. Also, each call produces its [`call_stamp`](https://github.com/MaciejBaj/pallet-contracts/blob/escrow-contracts/src/escrow_exec.rs#L54), which contains the info about the storage hash before & after execution. After the call execution, all of the storage changes are reverted, as if the call would never happened. Proofs of the result is generated by hashing the output of the last call and the storage proof is constructed by merging `post_states` of all `call_stamps` together.  
- `revert` phase discards `deferred_storage_writes` and removes the `deferred_result`.
- `commit` phase moves the `deferred_storage_writes` to all affected by execution contracts. Note, that between the execution and commit phase state of a contract might have changed. Default behaviour instructs the execution to abort the commit phase and invoke the revert phase instead when that happens. This can be changed by setting the config constant `WhenStateChangedForceTry = true`.

Having said that, there is two Gateway pallets:

##### `ContractsGateway` for parachains with Contracts

`ContractsGateway` pallet integrates with [`Contracts`](https://github.com/paritytech/substrate/tree/master/frame/contracts) pallet, and is modelled to provide the same developer experience as using the contracts pallet.
- `multistep_call`: executes attached code as a regular contract on that Parachan and/or calls existing contract (also recursively) on that Parachain. Implements `contracts::put_code`, `contracts::instantiate`, `contracts::bare_call`, `contracts::terminate`. After execution phase still no changes are added to the affected existent contracts.
- `rent_projection`: implements `contracts::rent_projection`.
- `get_storage`: implements `contracts::get_storage`.

##### `RuntimeGateway` for parachains without Contracts

`RuntimeGateway` pallet adds a possibility for parachains that do not implement [`Contracts`](https://github.com/paritytech/substrate/tree/master/frame/contracts) pallet, to provide a way of executing attached code in the context of that parachain and secure the results reveal and storage writes by escrow account.
- `multistep_call`: executes attached code as it would be a regular contract on that Parachan. This Only functions from the supported standards can be included in attached contracts. Implements its own version of [runtime sandboxed execution](https://github.com/MaciejBaj/pallet-contracts/blob/escrow-contracts/src/wasm/runtime_escrow.rs) of WASM code.
- `rent_projection`: not implemented yet.
- `get_storage`: not implemented yet.

Both Contract and Execution Gateways is intended to work within the [Gateway Circuit](https://github.com/t3rn/t3rn#gateway-circuit) which oversees, synchronises and secures the interoperable execution between Parachains involved and communicates with Gateway API.


#### Use
In this repository, both `escrow-gateway` & `escrow-pallet-balances` are installed as part of the demonstration node - `demo-runtime`, where the Escrow Gateway is one very few connected pallets. This is extended after [substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template). To start demonstration node run `bash run-node-full.sh`. The node runs on a default for Substrate `ws-port = 9944` & `http-port = 9933`. 

##### Front-end
You can verify the Escrow Gateway running demo frontend app from [front-end](./frontend) directory, or using the `Extrinsics` tool of [@polkadot/apps](https://github.com/polkadot-js/apps) GUI which is hosted on https://polkadot.js.org/apps/#/extrinsics. Remember to select a "local node" from the left-menu dropdown.

#### Installation

Please follow independent installation manuals for [`contracts-gateway`](pallets/contracts-gateway) and [`runtime-gateway`](pallets/runtime-gateway) depending on needs. Both of the pallets are installed as part of the [`demo-runtime`](./node).

#### API
The `escrow-gateway-api` pallet is intended to provide the way for a parachain to connect with the t3rn network operated by Gateway Circuit (scheduled to be implemented in [following development phases](../roadmap/following_development_phases.md)) or any other trusted service holding the 
authorisation keys.

As of now, `escrow_gateway` doesn't implement [Custom RPC](https://substrate.dev/recipes/3-entrees/custom-rpc.html). This might change in the next milestones. 

### Testing Guide

#### Unit Tests

Both `contracts-gateway` and `runtime-gateway` come with unit tests. Module instantiation is complex as the gateway introduces a dependency on the contracts and takes place in `src/mock.rs`.  

Unit tests cover scenarios checking interaction between execution phases and running the test on couple of WASM contracts:
- `returns_from_start_fn.wasm` - checking the correct output and proofs
- `transfer_return_code.wasm` - checking the correct output and transfers from within a contract.
- `storage_size.wasm` - checking setting of the input and deferred storage writes to a contract.
- `storage_runtime_demo.wasm` - (only `runtime-gateway`) dispatching calls to runtime as well as `deposit_event` on a demonstrative pallet that stores values and allows for multiple transformations on it - [weights](pallets/contracts-gateway/fixtures/storage_runtime_demo.wat).

To execute the unit test, type: 
```shell script
cargo test -- --nocapture
```

_While running tests, you may want to change the `debug::info!` to `println!` messages, like for `multistep_call` message:_

```rust
/// Change debug::info! to println! for test debugging.
// debug::info!("DEBUG multistep_call -- escrow_engine.execute  {:?}", exec_res);
println!("DEBUG multistep_call -- escrow_engine.execute {:?}", exec_res);
```

#### Integration Tests
Both `RuntimeGateway` and `ContractsGateway` come with the integration tests. 

Integration tests run different integration scenarios against running Substrate node (either `demo-runtime` or `full-node`) connecting with its Call API dedicated for extrinsics. 

To run the integration tests against the tiny node:
1. Build & run a `demo-runtime` with `bash run-node-tiny.sh`.
1. Execute integration tests against `ws:9944` default port: `cd test-integration && npm test:tiny` or `cd test-integration && npm test:full`.

###### [Execute multi-step transaction](tests/multistep_call.spec.js)
Uses Alice's account as the one that already has some positive balance on it allowing to put_code and instantiate contract. Also, by default Alice is registered as escrow account - sudo user with authorisation for calls. 

That Escrow Account sends the signed transaction against the `multistep_call` API containing valid example code and checks the correct results of that execution by retrieving the events out of substrate node - there should be one from the contracts pallet (code is stored_ and one from escrow gateway pallet - mutlistep_call result. 

### Please refer to the [Gateway specification](../specification/gateway_standalone.md) to find details on the future offer, intended shape and [Development Roadmap](../roadmap/initial_development_phase.md). 


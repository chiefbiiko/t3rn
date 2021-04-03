**todo**

+ unit testin nascent contract-registry storage
  + TODO: how2 name cargo test cases?? rename tes cases!!!!
+ documentin ideas for offchain storage - substrate archive -?
+ figure out how to iterate all contracts & all `contracts[requester]`
  + probly just iterate over the off-chain index (aka substrate-archive)
  + ITERATING FRAME STORAGE: [StoragePrefixedMap#iter_values](https://substrate.dev/rustdocs/v3.0.0/frame_support/storage/trait.StoragePrefixedMap.html#method.iter_values)
+ have the fetch_contracts RPC method use that iteration mech

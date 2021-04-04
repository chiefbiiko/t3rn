**todo**

+ unit testin nascent contract-registry storage
  + TODO: complete outlined test cases
  + check `assert_storage_noop!`
+ offchain storage - substrate archive on the side ?!
+ figure out how to iterate all contracts & all `contracts[requester]`
  + ITERATING FRAME STORAGE: [StoragePrefixedMap#iter_values](https://substrate.dev/rustdocs/v3.0.0/frame_support/storage/trait.StoragePrefixedMap.html#method.iter_values)
+ have the fetch_contracts RPC method serve contracts from substrate-archive
  
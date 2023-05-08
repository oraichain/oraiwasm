# oraiwasm

Oraichain smart contracts

### Build smart contracts

```bash
# build a smart contract
cw-build package/price/datasource_eth

# if need customize std, using this command
cargo install xargo

# run test a function
cargo test --lib -p nft -- --exact contract::tests::query_tokens_by_owner --show-output
```

## Handling smart contracts

The detailed tutorials for the smart contracts are located [here](https://oraiwasm.web.app/)

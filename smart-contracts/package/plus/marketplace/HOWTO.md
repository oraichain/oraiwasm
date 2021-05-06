# How To

This is a guide on how to setup your own set of contracts on the hackatom-wasm chain in conjunction with the marketplace. If you want to deploy the contracts on a local chain, please follow the [CosmWasm docs](https://docs.cosmwasm.com/getting-started/setting-env.html#run-local-node-optional) for instructions on how to set up a local node.

You will need two accounts with some tokens from the [faucet](https://five.hackatom.org/resources). Otherwise, you won't be able to upload the smart contracts to the hackatom-wasm chain.

> :information_source: **If you already have accounts with funds, you can skip this step.**

```shell
# Create accounts and save mnemonics
wasmcli keys add client
wasmcli keys add partner
```

## Building the Contracts

We need to build three smart contracts in total:

* `cw20-base` for buying tokens,
* `cw721-base` for selling tokens and withdrawing offerings
* and `marketplace`.

```shell
# Make sure all of your contracts actually build using
cargo wasm

# Switch into the hackatom_v/ directory and use the workspace-optimizer v0.10.4 to build your contracts
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.10.4
```

Once the workspace-optimizer is done, the `.wasm` files will be put into the `artifacts/` directory.

## Uploading the Contracts

Now that we've built our contracts, we need to upload them to the blockchain.

> :information_source: In order to avoid confusion, run `wasmcli query wasm list-code` after each individual upload to get the contract ID. You will be needing the IDs in the next step.

```shell
wasmcli tx wasm store artifacts/cw20_base.wasm --from client --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y
wasmcli tx wasm store artifacts/cw721_base.wasm --from client --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y
wasmcli tx wasm store artifacts/marketplace.wasm --from client --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y
```

## Instantiating the Contracts

Now that we've uploaded our contracts to the blockchain, we need to instantiate them individually using the IDs we got from uploading.

```shell
# cw20-base initialization
# - name: Your custom name of your CW20 contract
# - symbol: All upper case symbol of your CW20 token, must be 3-6 characters long (i.e. ATOM)
# - decimals: Number of decimal places for your tokens (i.e. 3 -> 1.xxx ATOM)
# - initial balances: Array with one or more accounts to give some tokens on contract initialization
#   - address: Account address to receive tokens
#   - amount: Amount of tokens the address should receive
# - mint: Object holding the minter address
#   - minter: Account address of the CW20 token minter
wasmcli tx wasm instantiate <CW20_BASE_CONTRACT_ID> '{
  "name": "<INSERT_NAME>",
  "symbol": "<INSERT_ALL_UPPER_CASE_SYMBOL>",
  "decimals": <INSERT_NUM_OF_DECIMAL_PLACES_FOR_TOKEN>,
  "initial_balances": [
    {
      "address": "<INSERT_ACCOUNT_ADDR>",
      "amount": "<INSERT_AMOUNT>"
    }
  ],
  "mint": {
    "minter": "<INSERT_MINTER_ADDR>"
  }
}' --label "cw20-base" --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y --from client

# cw721-base initialization
# - name: Your custom name of your CW721 contract
# - symbol: All upper case symbol of your CW721 token, must be 3-6 characters long (i.e. ATOM)
# - minter: Account address of the CW721 token minter
wasmcli tx wasm instantiate <CW721_BASE_CONTRACT_ID> '{
  "name": "<INSERT_NAME>",
  "symbol": "<INSERT_ALL_UPPER_CASE_SYMBOL>",
  "minter": "<INSERT_MINTER_ACCOUNT_ADDR>"
}' --label "cw721-base" --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y --from client

# marketplace initialization
# - name: Your custom name of your marketplace contract
wasmcli tx wasm instantiate <MARKETPLACE_CONTRACT_ID> '{
  "name": "<INSERT_NAME>"
}' --label "marketplace" --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y --from client
```

Once instantiated, you can use `wasmcli query wasm list-contract-by-code <CONTRACT_ID>` to query contract info.

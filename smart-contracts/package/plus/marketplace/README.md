# Marketplace Smart Contract

The marketplace smart contracts provides a generic platform used for selling and buying CW721 tokens with CW20 tokens. It maintains a list of all current offerings, including the seller's address, the token ID put up for sale, the list price of the token and the contract address the offerings originated from. This ensures maximum visibility on a per-sale instead of a per-contract basis, allowing users to browse through list of offerings in one central place.

## Requirements

* [Go `v1.14+`](https://golang.org/)
* [Rust `v1.44.1+`](https://rustup.rs/)
* [Wasmd v0.11.1](https://github.com/CosmWasm/wasmd/tree/v0.11.1)
* [cosmwasm-plus v0.3.2](https://github.com/CosmWasm/cosmwasm-plus)
  * [cw20-base](https://github.com/CosmWasm/cosmwasm-plus/tree/master/contracts/cw20-base)
  * [cosmons](https://github.com/BlockscapeNetwork/hackatom_v/tree/master/contracts/cosmons)

## Setup Environment

1) Follow [the CosmWasm docs](https://docs.cosmwasm.com/getting-started/installation.html) to install `go v1.14+`, `rust v1.44.1+` and `wasmd v0.11.1`
2) Once you've built `wasmd`, use the `wasmcli` to join the `hackatom-wasm` chain.

> :information_source: If you want to deploy your own contracts on your own chain, check out the [HOWTO](HOWTO.md).

```shell
wasmcli config chain-id hackatom-wasm
wasmcli config indent true
wasmcli config keyring-backend test
wasmcli config node https://rpc.cosmwasm.hub.hackatom.dev:443
wasmcli config output json
wasmcli config trust-node true
```

3) Create an account with some tokens from the [faucet](https://five.hackatom.org/resources). Otherwise, you won't be able to make any transactions.

> :information_source: **If you already have an account with funds, you can skip this step.**

```shell
# Create account and save mnemonics
wasmcli keys add myacc
```

4) Before you can buy or sell CW721 tokens, you will need some CW20 tokens. You can get them from our faucet: `POST 3.121.232.142:8080/faucet`

Example payload:

```json
{
  "address": "<INSERT_ACCOUNT_ADDRESS>"
}
```

## Contract Addresses

| Contract        | Address                                       |
|:----------------|:----------------------------------------------|
| marketplace     | cosmos1knqr4zclds5zhn5khkpexkd7nctwe8z0s2qer4 |
| cw20-base       | cosmos1kfz3mj84atqjld0ge9eccujvqqkqdr4qqs9ud7 |
| cosmons (cw721) | cosmos1zhh3m9sg5e2qvjgwr49r79pf5pt65yuxvs7cs0 |

## Messages

### Sell CW721 Token

Puts an NFT token up for sale.

> :warning: The seller needs to be the owner of the token to be able to sell it.

```shell
# Execute send_nft action to put token up for sale for specified list_price on the marketplace
wasmcli tx wasm execute <CW721_BASE_CONTRACT_ADDR> '{
  "send_nft": {
    "contract": "<MARKETPLACE_CONTRACT_ADDR>",
    "token_id": "<TOKEN_ID>",
    "msg": "BASE64_ENCODED_JSON --> { "list_price": { "address": "<INSERT_CW20_CONTRACT_ADDR>", "amount": "<INSERT_AMOUNT_WITHOUT_DENOM>" }} <--"
  }
}' --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y --from client
```

### Withdraw CW721 Token Offering

Withdraws an NFT token offering from the global offerings list and returns the NFT token back to its owner.

> :warning: Only the token's owner/seller can withdraw the offering. This will only work after having used `sell_nft` on a token.

```shell
# Execute withdraw_nft action to withdraw the token with the specified offering_id from the marketplace
wasmcli tx wasm execute <MARKETPLACE_CONTRACT_ADDR> '{
  "withdraw_nft": {
    "offering_id": "<INSERT_OFFERING_ID>"
  }
}' --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y --from client
```

### Buy CW721 Token

Buys an NFT token, transferring funds to the seller and the token to the buyer.

> :warning: This will only work after having used `sell_nft` on a token.

```shell
# Execute send action to buy token with the specified offering_id from the marketplace
wasmcli tx wasm execute <CW20_BASE_CONTRACT_ADDR> '{
  "send": {
    "contract": "<MARKETPLACE_CONTRACT_ADDR>",
    "amount": "<INSERT_AMOUNT>",
    "msg": "BASE64_ENCODED_JSON --> { "offering_id": "<INSERT_OFFERING_ID>" } <--"
  }
}' --gas-prices="0.025ucosm" --gas="auto" --gas-adjustment="1.2" -y --from client
```

## Queries

### Query Offerings

Retrieves a list of all currently listed offerings.

```shell
wasmcli query wasm contract-state smart <MARKETPLACE_CONTRACT_ADDR> '{
  "get_offerings": {}
}'
```

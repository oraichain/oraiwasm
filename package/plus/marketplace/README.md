# Marketplace Smart Contract

The marketplace smart contracts provides a generic platform used for selling and buying CW721 tokens with CW20 tokens. It maintains a list of all current offerings, including the seller's address, the token ID put up for sale, the list price of the token and the contract address the offerings originated from. This ensures maximum visibility on a per-sale instead of a per-contract basis, allowing users to browse through list of offerings in one central place.

## Requirements

- [Rust `v1.53.0+`](https://rustup.rs/)
- [Wasmd v0.13.2](https://github.com/CosmWasm/wasmd/tree/v0.11.1)

## Setup Environment

> :docker_compose: If you don't want to setup rust you can use docker-compose

```shell
# link required contracts
mkdir contract && cd contract
ln -s ../../../ow20/artifacts ow20
```

## Contract Addresses

| Contract    | Address     |
| :---------- | :---------- |
| marketplace | marketplace |
| ow20        | ow20        |
| ow721       | ow721       |

> :warning: You need to put ow20 and ow721 artifacts folders into artifacts/contract before interacting.

### Start Simulating

`simulate package/plus/marketplace/artifacts/marketplace.wasm -b '{"address":"fake_receiver_addr","amount":"300000"}' -c contract`

### Init Smart Contracts

```shell
# Init marketplace
cosmwasm-simulate init marketplace '{
  "name": "nft market"
}'

# Init ow721
cosmwasm-simulate init ow721 '{
  "minter": "marketplace",
  "name": "nft collection",
  "symbol": "NFT"
}'

```

### Sell CW721 Token

Puts an NFT token up for sale.

> :warning: The seller needs to be the owner of the token to be able to sell it.

```shell
# Execute mint to create new NFT Token with fake_receiver_addr account
cosmwasm-simulate handle marketplace `{
  "mint_nft": {
    "contract": "ow721",
    "msg": {
      "mint": {
        "description": "nft desc",
        "image": "https://ipfs.io/ipfs/QmWCp5t1TLsLQyjDFa87ZAp72zYqmC7L2DsNjFdpH8bBoz",
        "name": "nft rare",
        "owner": "fake_receiver_addr",
        "token_id": "123456"
      }
    }
  }
}`

# Execute send_nft action to put token up for sale for specified_price on the marketplace
# msg in base64 format: eyJsaXN0X3ByaWNlIjp7ImFkZHJlc3MiOiJvdzIwIiwiYW1vdW50IjoiNTAifX0=
cosmwasm-simulate handle ow721 `{
  "send_nft": {
    "contract": "marketplace",
    "msg": {"price": "50"},
    "token_id": "123456"
  }
}`

```

### Query Offerings

Retrieves a list of all currently listed offerings.

```shell
cosmwasm-simulate query marketplace '{
  "get_offerings": {}
}'
```

### Withdraw CW721 Token Offering

Withdraws an NFT token offering from the global offerings list and returns the NFT token back to its owner.

> :warning: Only the token's owner/seller can withdraw the offering. This will only work after having used `sell_nft` on a token.

```shell
# Execute withdraw_nft action to withdraw the token with the specified offering_id from the marketplace
cosmwasm-simulate handle marketplace '{
  "withdraw_nft": {
    "offering_id": "<INSERT_OFFERING_ID>"
  }
}'
```

### Buy CW721 Token

Buys an NFT token, transferring funds to the seller and the token to the buyer.

> :warning: This will only work after having used `sell_nft` on a token.

```shell
# Execute send action to buy token with the specified offering_id from the marketplace
# msg in base64 format: eyJvZmZlcmluZ19pZCI6IjEifQ==
cosmwasm-simulate handle marketplace '{"buy_nft":{"offering_id":"1"}}'

```

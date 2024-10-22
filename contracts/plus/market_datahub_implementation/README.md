# Marketplace Smart Contract

The auction_nft smart contracts provides a generic platform used for selling and buying CW721 tokens with CW20 tokens. It maintains a list of all current auctions, including the seller's address, the token ID put up for sale, the list price of the token and the contract address the auctions originated from. This ensures maximum visibility on a per-sale instead of a per-contract basis, allowing users to browse through list of auctions in one central place.

## Requirements

- [Rust `v1.53.0+`](https://rustup.rs/)
- [Wasmd v0.13.2](https://github.com/CosmWasm/wasmd/tree/v0.11.1)

## Setup Environment

> :docker_compose: If you don't want to setup rust you can use docker-compose

## Contract Addresses

| Contract    | Address     |
| :---------- | :---------- |
| auction_nft | auction_nft |
| ow721       | ow721       |

> :warning: You need to put ow20 and ow721 artifacts folders into artifacts/contract before interacting.

### Start Simulating

`simulate package/plus/auction_nft/artifacts/auction_nft.wasm -b '{"address":"fake_receiver_addr","amount":"300000"}' -c contract`

### Init Smart Contracts

```shell
# Init auction_nft
cosmwasm-simulate init auction_nft '{
  "name": "nft market"
}'

# Init ow721
cosmwasm-simulate init ow721 '{
  "minter": "auction_nft",
  "name": "nft collection",
  "symbol": "NFT"
}'

```

### Sell CW721 Token

Puts an NFT token up for sale.

> :warning: The seller needs to be the owner of the token to be able to sell it.

```shell

# Execute ask_nft action to put token up for bid for specified_price on the auction_nft
# msg in base64 format: eyJsaXN0X3ByaWNlIjp7ImFkZHJlc3MiOiJvdzIwIiwiYW1vdW50IjoiNTAifX0=
cosmwasm-simulate handle ow721 `{
  "ask_nft": {
    "contract": "auction_nft",
    "msg": {"price": "50",cancel_fee: 1,start: 15, end: 140},
    "token_id": "123456"
  }
}`

```

### Query Offerings

Retrieves a list of all currently listed auctions.

```shell
cosmwasm-simulate query auction_nft '{
  "get_auctions": {}
}'
```

### Withdraw CW721 Token Offering

Withdraws an NFT token auction from the global auctions list and returns the NFT token back to its owner.

> :warning: Only the token's owner/seller can withdraw the auction. This will only work after having used `sell_nft` on a token.

```shell
# Execute withdraw_nft action to withdraw the token with the specified auction_id from the auction_nft
cosmwasm-simulate handle auction_nft '{
  "withdraw_nft": {
    "auction_id": "<INSERT_AUCTION_ID>"
  }
}'
```

### Buy CW721 Token

Buys an NFT token, transferring funds to the asker and the token to the bidder.

> :warning: This will only work after having used `ask_nft` on a token.

```shell
# Execute send action to bid token with the specified auction_id from the auction_nft
# msg in base64 format: eyJvZmZlcmluZ19pZCI6IjEifQ==
cosmwasm-simulate handle auction_nft '{"bid_nft":{"auction_id":"1"}}'

```

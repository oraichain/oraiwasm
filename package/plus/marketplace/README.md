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

### Sell CW721 Token

Puts an NFT token up for sale.

> :warning: The seller needs to be the owner of the token to be able to sell it.

```shell
# Execute send_nft action to put token up for sale for specified list_price on the marketplace
# msg: eyJsaXN0X3ByaWNlIjp7ImFkZHJlc3MiOiJvdzIwIiwiYW1vdW50IjoiNTAifX0=
cosmwasm-simulate ow721 `{
  "send_nft": {
    "contract": "marketplace",
    "msg": "base64({ "list_price": { "address": "ow20", "amount": "50" }})",
    "token_id": "123456",
  }
}`

# Execute receive_nft action to receive token for sale for specified list_price on the marketplace
# msg: eyJsaXN0X3ByaWNlIjp7ImFkZHJlc3MiOiJvdzIwIiwiYW1vdW50IjoiMSJ9fQ==
cosmwasm-simulate marketplace '{
  "receive_nft": {
     "msg": "base64({ "list_price": { "address": "ow20", "amount": "1" }})",
     "sender": "fake_sender_addr",
     "token_id": "123456"
  }
}'
```

## Queries

### Query Offerings

Retrieves a list of all currently listed offerings.

```shell
cosmwasm-simulate marketplace '{
  "get_offerings": {}
}'
```

### Withdraw CW721 Token Offering

Withdraws an NFT token offering from the global offerings list and returns the NFT token back to its owner.

> :warning: Only the token's owner/seller can withdraw the offering. This will only work after having used `sell_nft` on a token.

```shell
# Execute withdraw_nft action to withdraw the token with the specified offering_id from the marketplace
cosmwasm-simulate marketplace '{
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
cosmwasm-simulate ow20  '{
  "send": {
    "contract": marketplace,
    "amount": "<INSERT_AMOUNT>",
    "msg": "base64({ "offering_id": "<INSERT_OFFERING_ID>" })"
  }
}'

cosmwasm-simulate marketplace '{
  "receive": {
     "sender": "fake_sender_addr",
     "amount": "1",
     "msg": "base64({"offering_id":"<INSERT_OFFERING_ID>"})",
  }
}'
```

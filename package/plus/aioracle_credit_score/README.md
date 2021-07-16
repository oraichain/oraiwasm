## How to run and query

current credit score aioracle smart contract: orai1xrwwv8d6dnz3ulxgwqjqdxprxxhu9jwrmsnp50

All commands below are used in the repository: https://github.com/oraichain/oraicli

Firstly, clone the oraicli repo, type: ```yarn``` to get the dependencies, and add a .env file below:

```
SEND_MNEMONIC=
URL=http://testnet-lcd.orai.io
DENOM=orai
CHAIN_ID=Oraichain-testnet
```
where ```SEND_MNEMONIC``` is your Oraichain testnet account. It should have some balances, because the data source requires 10^-5 ORAI to be executed. Please use the [testnet wallet app](https://testnet-wallet.web.app/) to create a new wallet, and use the [testnet faucet](https://testnet-faucet.web.app/) to get some tokens for testinng.

### Query

#### Query the latest batch of credit scores

Example: ```yarn oraicli wasm query orai1xrwwv8d6dnz3ulxgwqjqdxprxxhu9jwrmsnp50 --input '{"query_latest":{}}'```

#### Query a specific batch of credit scores

Example: ```yarn oraicli wasm query orai1xrwwv8d6dnz3ulxgwqjqdxprxxhu9jwrmsnp50 --input '{"query_specific":{"epoch":3}}'```

#### Query a list of batch of credit scores

Example: ```yarn oraicli wasm query orai1xrwwv8d6dnz3ulxgwqjqdxprxxhu9jwrmsnp50 --input '{"query_list":{"offset":1,"limit":100,"order":1}}'```

### Run

Example: ```yarn oraicli wasm execute orai1xrwwv8d6dnz3ulxgwqjqdxprxxhu9jwrmsnp50 --input '{"oracle_handle":{"msg":{"create_ai_request":{"validators":["orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt"],"input":"{\"perPage\":\"10\",\"pageNum\":\"1\",\"epoch\":\"4\"}"}}}}' --amount 10```

In the above command, you can edit the per page, page num and epoch field to adjust the credit score result you get.

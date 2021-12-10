## phrases:

- a number of members act as dealers send commit and secret_row encrypted by node[m] public key and commit share
- all node[m] m in [0..total] will check the commit then aggreate secret_row shared to form the secret key share and commit public key share
- each round will wait for all commits that are valid by checking with public key share, then combine the valid sig to produce randomness

## deploy contract

```bash
# on MacOS need to add CFLAGS="-I$(xcrun --show-sdk-path)/usr/include" CC="gcc -m32" cargo build ...

yarn oraicli wasm deploy package/plus/vrfdkg/artifacts/vrfdkg.wasm --input '{"members": [{ "address": "orai15kjxk4yr4del2tjvm8vrz7ghzm8nflazsjq6ar", "pubkey": "A2JyjvCWNpj83BR+UbXkBSbSp7nW71V4hg4YlhqxZRJA" },{ "address": "orai107myn6xlt90rh8gky7zms5d082gyv759sxvmah", "pubkey": "AmgbPq+M9/qELxUEceBWrZ+Hbn1FoVAH6zZpWW5UVlWU" },{ "address": "orai1duexpl5m3vc6sjk4hm3ctlg5f67rfa6zgrfq25", "pubkey": "AnlknVYRFr6C//taaG6BYeInbdHBfB6jh4cxvU9GPOUi" },{"address":"orai1f7lqjezdrfgl9j978ut5u404dlk0tymqmzpmeu","pubkey":"ApcsaOP6SjoEtBKHXx9/NPBoBdN/C/h9ymxR0gBZIsuj"},{"address":"orai1fyjznczdrk3l7smre6g57fgm84n6qm6dvtj2l2","pubkey":"Ajv3EyhURIixYumrmSX4vujzCJXBbZy+8PBRUpiT0F86"}],"threshold": 2, "dealer": 3}' --label 'vrfdkg' --gas 6000000 --env .env.development

```

## simulate the contract

```bash
cosmwasm-simulate package/plus/vrfdkg/artifacts/vrfdkg.wasm -b '{"address":"orai15kjxk4yr4del2tjvm8vrz7ghzm8nflazsjq6ar","amount":"10"}' -b '{"address":"orai107myn6xlt90rh8gky7zms5d082gyv759sxvmah","amount":"10"}' -b '{"address":"orai1duexpl5m3vc6sjk4hm3ctlg5f67rfa6zgrfq25","amount":"10"}' -b '{"address":"orai1f7lqjezdrfgl9j978ut5u404dlk0tymqmzpmeu","amount":"10"}' -b '{"address":"orai1fyjznczdrk3l7smre6g57fgm84n6qm6dvtj2l2","amount":"10"}'
```

# !usr/bin/env bash
set -eux

npm install -g @oraichain/cwtools --ignore-scripts
mkdir artifacts
cwtools build package/plus/market_implementation -o artifacts/market_implementation 
cwtools build package/plus/market_1155_implementation -o artifacts/market_1155_implementation 

cat << EOF > artifacts/market_1155_implementation/initMsg.json
{
  "name": "airight market 1155 implementation v3.2",
  "denom": "orai",
  "fee": 20,
  "governance": "orai14tqq093nu88tzs7ryyslr78sm3tzrmnpem6fak",
  "auction_duration": "3600000",
  "step_price": 1,
  "admin": [
    "orai1w0emvx75v79x2rm0afcw7sn8hqt5f4fhtd3pw7",
    "orai1zsqaw40hcj4hk7r2g3xz864gda9vpq3ze9vpxc"
  ]
}
EOF

cat << EOF > artifacts/market_implementation/initMsg.json
{
  "name": "airight market 721 implementation v3.2",
  "denom": "orai",
  "fee": 20,
  "governance": "orai14tqq093nu88tzs7ryyslr78sm3tzrmnpem6fak",
  "auction_duration": "3600000",
  "step_price": 1,
  "max_royalty": 1000000000,
  "max_decimal_point": 1000000000
}
EOF


# rm -rf artifacts



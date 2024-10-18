## How to run

```bash
# build contract
./optimize.sh package/price/dsource_coingecko true
./optimize.sh package/plus/aioracle_pricefeed true

# link contract references
ln -s $PWD/package/price/dsource_coingecko/artifacts/ $PWD/package/plus/aioracle_pricefeed/artifacts/contract/dsource_coingecko

# simulate
./simulate package/plus/aioracle_pricefeed/artifacts/aioracle_pricefeed.wasm -c contract
```

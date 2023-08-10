const { readFileSync, existsSync, writeFileSync } = require("fs");
const { resolve } = require("path");
const downloadState = require("./downloadState");

(async () => {
  const registry = JSON.parse(
    readFileSync(resolve(__dirname, "./registry.json"))
  );
  const addresses = Object.values(registry);

  const storagePromises = addresses.map(async (address) => {
    const filePath = resolve(
      __dirname,
      "../tests/testdata/",
      `${address}.json`
    );

    let old_data = [];
    let nextKey;
    if (existsSync(filePath)) {
      old_data = JSON.parse(readFileSync(filePath));
      nextKey = old_data.pop()[0];
    }
    const {
      contract_info: { code_id },
    } = await fetch(
      `https://lcd.orai.io/cosmwasm/wasm/v1/contract/${address}`
    ).then((res) => res.json());
    const { data } = await fetch(
      `https://lcd.orai.io/cosmwasm/wasm/v1/code/${code_id}`
    ).then((res) => res.json());

    writeFileSync(
      resolve(__dirname, `../tests/wasm/${address}`),
      Buffer.from(data, "base64")
    );

    return downloadState(
      address,
      (chunks) => {
        chunks.forEach(function ({ key, value }) {
          old_data.push([Buffer.from(key, "hex").toString("base64"), value]);
        });

        writeFileSync(filePath, JSON.stringify(old_data));
      },
      nextKey,
      50000
    );
  });

  await Promise.all(storagePromises);
})();

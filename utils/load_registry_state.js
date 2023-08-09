const { readFileSync, existsSync, writeFileSync } = require("fs");
const { resolve } = require("path");
const downloadState = require("./downloadState");

async function registry() {
  const data = JSON.parse(
    readFileSync(resolve(__dirname, "../tests/testdata/registry.json"))
  );

  const storage = data.data.storages;
  const implementation = data.data.implementations;
  const collections = data.data.collections;

  return { storage, implementation, collections };
}

(async () => {
  const { storage, implementation, collections } = await registry();

  const storagePromises = storage.map((storage) => {
    const [_name, address] = storage;
    const filePath = resolve(
      __dirname,
      "../tests/testdata/",
      `${address}.json`
    );
    const isExist = existsSync(filePath);
    return downloadState(
      address,
      (chunks) => {
        let old_data = [];
        if (isExist) {
          old_data = JSON.parse(readFileSync(filePath));
        }
        chunks.forEach(function({ key, value }) {
          old_data.push([Buffer.from(key, "hex").toString("base64"), value]);
        });

        writeFileSync(filePath, JSON.stringify(old_data));
      },
      50000
    );
  });

  await Promise.all(storagePromises);
})();

module.exports.registry = registry;

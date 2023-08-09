const { SimulateCosmWasmClient } = require("@oraichain/cw-simulate");
const { readFileSync } = require("fs");
const { resolve } = require("path");

const client = new SimulateCosmWasmClient({
  chainId: "Oraichain",
  bech32Prefix: "orai",
  metering: true,
});

describe("Test load data", () => {
  it("should load data successfully", async () => {
    const admin = "orai1zsqaw40hcj4hk7r2g3xz864gda9vpq3ze9vpxc";
    const contractTest = "orai14tqq093nu88tzs7ryyslr78sm3tzrmnpem6fak";
    const { codeId } = await client.upload(
      admin,
      readFileSync(
        resolve(__dirname, "../artifacts/market_hub/market_hub.wasm")
      )
    );
    await client.loadContract(
      contractTest,
      {
        codeId,
        label: "market_hub",
        admin,
        creator: admin,
      },
      JSON.parse(
        readFileSync(
          resolve(
            __dirname,
            "./testdata/orai14tqq093nu88tzs7ryyslr78sm3tzrmnpem6fak.json"
          )
        )
      )
    );

    const registry = await client.queryContractSmart(contractTest, {
      registry: {},
    });
    expect(registry).toBeTruthy();
  });
});

const { readFileSync } = require("fs");
const { resolve } = require("path");
const { SimulateCosmWasmClient } = require("@oraichain/cw-simulate");

const admin = "orai14vcw5qk0tdvknpa38wz46js5g7vrvut8lk0lk6";

async function setUpMarketPlaceEnviroment() {
  const registry = JSON.parse(
    readFileSync(resolve(__dirname, "../utils/registry.json"))
  );
  const client = new SimulateCosmWasmClient({
    chainId: "Oraichain",
    bech32Prefix: "orai",
    metering: true,
  });

  client.app.bank.setBalance(admin, [{ amount: "10000000000", denom: "orai" }]);

  const codeIdPromises = Object.entries(registry).map(([_label, address]) => {
    return client.upload(
      admin,
      readFileSync(resolve(__dirname, `./wasm/${address}`)),
      "auto"
    );
  });

  const codeIds = (await Promise.all(codeIdPromises)).map(
    (result) => result.codeId
  );

  const loadPromises = Object.entries(registry).map(
    ([name, address], index) => {
      const data = JSON.parse(
        readFileSync(
          resolve(__dirname, `./testdata/${address}.json`)
        ).toString()
      );
      return client.loadContract(
        address,
        {
          codeId: codeIds[index],
          label: name,
          admin,
          creator: admin,
        },
        data
      );
    }
  );
  await Promise.all(loadPromises);

  const market1155ImplInitMsg = JSON.parse(
    readFileSync(
      resolve(
        __dirname,
        "../artifacts/market_1155_implementation/initMsg.json"
      ).toString()
    )
  );
  const marketImplInitMsg = JSON.parse(
    readFileSync(
      resolve(__dirname, "../artifacts/market_implementation/initMsg.json")
    ).toString()
  );

  const [market1155, market721] = await Promise.all([
    client
      .upload(
        admin,
        readFileSync(
          resolve(
            __dirname,
            "../artifacts/market_1155_implementation/market_1155_implementation.wasm"
          )
        ),
        "auto"
      )
      .then(async ({ codeId }) => {
        return await client.instantiate(
          admin,
          codeId,
          market1155ImplInitMsg,
          "market_1155_impl",
          "auto"
        );
      }),
    client
      .upload(
        admin,
        readFileSync(
          resolve(
            __dirname,
            "../artifacts/market_implementation/market_implementation.wasm"
          )
        ),
        "auto"
      )
      .then(async ({ codeId }) => {
        return await client.instantiate(
          admin,
          codeId,
          marketImplInitMsg,
          "market_impl",
          "auto"
        );
      }),
  ]);

  registry["market_721_impl"] = market721.contractAddress;
  registry["market_1155_impl"] = market1155.contractAddress;

  return { client, registry };
}

describe("Test marketplace buy for", () => {
  let client, registry, offering1155, offering721;
  const ownerNFT = "orai14vcw5qk0tdvknpa38wz46js5g7vrvut8lk0lk6";
  const admin1155 = "orai1zsqaw40hcj4hk7r2g3xz864gda9vpq3ze9vpxc";
  const mintForAddress = "orai1w0emvx75v79x2rm0afcw7sn8hqt5f4fhtd3pw7";
  const buyForAddress = "orai149uraxsj6vqqugzth8drw8akjcuktkfl3tafue";

  beforeAll(async () => {
    ({ client, registry } = await setUpMarketPlaceEnviroment());
    client.app.bank.setBalance(ownerNFT, [
      { amount: "10000000000", denom: "orai" },
    ]);
    client.app.bank.setBalance(admin1155, [
      { amount: "10000000000", denom: "orai" },
    ]);
    [offering1155, offering721] = await Promise.all([
      client.queryContractSmart(registry["market_1155_storage"], {
        msg: { get_offerings: {} },
      }),
      client.queryContractSmart(registry["market_offering_storage"], {
        offering: { get_offerings: {} },
      }),
    ]);
  }, 30000);

  it("Setup successfull", async () => {
    console.log("success");
  });

  it("Should return registry & adminlist from market_hub", async () => {
    const [registryData, canExecute] = await Promise.all([
      client.queryContractSmart(registry["market_hub"], {
        registry: {},
      }),
      client.queryContractSmart(registry["market_hub"], {
        can_execute: {
          sender: "orai14vcw5qk0tdvknpa38wz46js5g7vrvut8lk0lk6",
        },
      }),
    ]);
    expect(canExecute).toBeTruthy();
    expect(registryData).toBeTruthy();
  });

  it("should add the the new implement in market_hub", async () => {
    await client.execute(
      admin,
      registry["market_hub"],
      {
        update_implementation: {
          implementation: registry["market_1155_impl"],
        },
      },
      "auto"
    );

    await client.execute(
      admin,
      registry["market_hub"],
      {
        update_implementation: {
          implementation: registry["market_721_impl"],
        },
      },
      "auto"
    );
    const registryData = await client.queryContractSmart(
      registry["market_hub"],
      {
        registry: {},
      }
    );

    expect(registryData.implementations.length).toBe(5);
  });

  it("should change minter in collection", async () => {
    await Promise.all([
      client.execute(ownerNFT, registry["ow1155"], {
        change_minter: { minter: registry["market_1155_impl"] },
      }),
      client.execute(ownerNFT, registry["ow721"], {
        change_minter: { minter: registry["market_721_impl"] },
      }),
    ]);

    const [minter1155] = await Promise.all([
      client.queryContractSmart(registry["ow1155"], {
        minter: {},
      }),
    ]);

    expect(minter1155).toEqual(registry["market_1155_impl"]);
  });

  it("should mint for another user", async () => {
    await Promise.all([
      client.execute(
        admin1155,
        registry["market_1155_impl"],
        {
          mint_for_nft: {
            contract_addr: registry["ow1155"],
            creator: mintForAddress,
            creator_type: "creator",
            mint: {
              mint: {
                to: mintForAddress,
                token_id: "260900",
                value: "2000",
              },
            },
          },
        },
        "auto"
      ),
      client.execute(
        admin1155,
        registry["market_721_impl"],
        {
          mint_nft: {
            contract_addr: registry["ow721"],
            creator: mintForAddress,
            creator_type: "creator",
            mint: {
              mint: {
                token_id: "260900",
                owner: mintForAddress,
                name: "TEST_TEST",
                image: "https://www.facebook.com/puongnhnbo",
              },
            },
          },
        },
        "auto"
      ),
    ]);

    const [token721Info, balance1155] = await Promise.all([
      client.queryContractSmart(registry["ow721"], {
        nft_info: {
          token_id: "260900",
        },
      }),
      client.queryContractSmart(registry["ow1155"], {
        balance: {
          owner: mintForAddress,
          token_id: "260900",
        },
      }),
    ]);

    expect(token721Info.name).toEqual("TEST_TEST");
    expect(balance1155.balance).toEqual("2000");
  });

  it("should buy for another user by native", async () => {
    const {
      id: offer_id_721,
      token_id: token_id_721,
      seller: seller_721,
      price,
    } = offering721.offerings.pop();
    const {
      id: latest_offering,
      token_id,
      per_price,
      seller,
    } = offering1155.pop();

    client.app.bank.setBalance(seller, [
      { amount: "10000000000", denom: "orai" },
    ]);

    client.app.bank.setBalance(seller_721, [
      { amount: "10000000000", denom: "orai" },
    ]);

    // Approve
    await Promise.all([
      client.execute(
        seller,
        registry["ow1155"],
        {
          approve_all: { operator: registry["market_1155_impl"] },
        },
        "auto"
      ),
      client.execute(
        seller_721,
        registry["ow721"],
        {
          approve_all: { operator: registry["market_721_impl"] },
        },
        "auto"
      ),
    ]);

    await Promise.all([
      client.execute(
        admin1155,
        registry["market_1155_impl"],
        {
          buy_nft: {
            offering_id: latest_offering,
            amount: "1",
            buyer: buyForAddress,
          },
        },
        "auto",
        "memo",
        [{ amount: per_price, denom: "orai" }]
      ),
      client.execute(
        admin1155,
        registry["market_721_impl"],
        {
          buy_nft: {
            offering_id: offer_id_721,
            buyer: buyForAddress,
          },
        },
        "auto",
        "memo",
        [{ amount: price, denom: "orai" }]
      ),
    ]);

    const [sellerBalance, buyForBalance, owner_token_721] = await Promise.all([
      client.queryContractSmart(registry["ow1155"], {
        balance: {
          owner: seller,
          token_id,
        },
      }),
      client.queryContractSmart(registry["ow1155"], {
        balance: {
          owner: buyForAddress,
          token_id,
        },
      }),
      client.queryContractSmart(registry["ow721"], {
        owner_of: {
          token_id: token_id_721,
        },
      }),
    ]);

    expect(sellerBalance.balance).toBe("0");
    expect(buyForBalance.balance).toBe("1");
    expect(owner_token_721.owner).toBe(buyForAddress);
  });

  it("should buy for another user by aiRight", async () => {
    const tokenIdPriceByAiRight = "15246";
    const owner = "orai1k8g0vlfmctyqtwahrxhudksz7rgrm6nsuwh8eq";
    const richAccount = "orai1xzmgjjlz7kacgkpxk5gn6lqa0dvavg8r9ng2vu";

    const [owner721, offering] = await Promise.all([
      client.queryContractSmart(registry["ow721"], {
        owner_of: {
          token_id: tokenIdPriceByAiRight,
        },
      }),
      client.queryContractSmart(registry["market_offering_storage"], {
        offering: {
          get_offering_by_contract_token_id: {
            contract: registry["ow721"],
            token_id: tokenIdPriceByAiRight,
          },
        },
      }),
    ]);

    const { id, token_id, price, contract_addr, seller } = offering;

    expect(owner721.owner).toBe(owner);

    await client.execute(
      owner,
      registry["ow721"],
      {
        approve_all: { operator: registry["market_721_impl"] },
      },
      "auto"
    );

    await client.execute(
      richAccount,
      registry["ai_right"],
      {
        send: {
          contract: registry["market_721_impl"],
          amount: price,
          msg: Buffer.from(
            JSON.stringify({
              buy_nft: {
                offering_id: parseInt(id),
                buyer: buyForAddress,
              },
            })
          ).toString("base64"),
        },
      },
      "auto"
    );

    const newOwner = await client.queryContractSmart(registry["ow721"], {
      owner_of: {
        token_id: tokenIdPriceByAiRight,
      },
    });

    expect(newOwner.owner).toBe(buyForAddress);
  });
});

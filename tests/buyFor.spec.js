const { readFileSync } = require("fs");
const { resolve } = require("path");
const { SimulateCosmWasmClient } = require("@oraichain/cw-simulate");

const admin = "orai14vcw5qk0tdvknpa38wz46js5g7vrvut8lk0lk6";

const market_hub = "orai14tqq093nu88tzs7ryyslr78sm3tzrmnpem6fak";
const market_offering_storage = "orai1hur7m6wu7v79t6m3qal6qe0ufklw8uckrxk5lt";
const market_1155_storage = "orai1v2psavrxwgh39v0ead7z4rcn4qq2cfnast98m9";
const market_rejected_storage = "orai1fp9lernzdwkg5z9l9ejrwjmjvezzypacspmw27";
const market_whitelist_storage = "orai1u4zqgyt8adq45a8xffc356dr8dqsny6merh0h0";
const market_721_payment_storage =
  "orai1ynvtgqffwgcxxx0hnehj4t7gsmv25nrr770s83";
const market_1155_payment_storage =
  "orai1l783x7q0yvr9aklr2zkpkpspq7vmxmfnndgl7c";
const market_auction_storage = "orai1u8r0kkmevkgjkeacfgh0jv268kap82af937pwz";
const market_ai_royalty_storage = "orai1s5jlhcnqc00hqmldhts5jtd7f3tfwmr4lfheg8";
const ow1155 = "orai1c3phe2dcu852ypgvt0peqj8f5kx4x0s4zqcky4";
const ow721 = "orai1ase8wkkhczqdda83f0cd9lnuyvf47465j70hyk";
let market_1155_impl, market_721_impl;

const setUpList = [
  ["market_hub", "orai14tqq093nu88tzs7ryyslr78sm3tzrmnpem6fak"],
  ["market_offering_storage", "orai1hur7m6wu7v79t6m3qal6qe0ufklw8uckrxk5lt"],
  ["market_1155_storage", "orai1v2psavrxwgh39v0ead7z4rcn4qq2cfnast98m9"],
  ["market_rejected_storage", "orai1fp9lernzdwkg5z9l9ejrwjmjvezzypacspmw27"],
  ["market_whitelist_storage", "orai1u4zqgyt8adq45a8xffc356dr8dqsny6merh0h0"],
  ["market_721_payment_storage", "orai1ynvtgqffwgcxxx0hnehj4t7gsmv25nrr770s83"],
  [
    "market_1155_payment_storage",
    "orai1l783x7q0yvr9aklr2zkpkpspq7vmxmfnndgl7c",
  ],
  ["market_auction_storage", "orai1u8r0kkmevkgjkeacfgh0jv268kap82af937pwz"],
  ["market_ai_royalty_storage", "orai1s5jlhcnqc00hqmldhts5jtd7f3tfwmr4lfheg8"],
  ["ow1155", "orai1c3phe2dcu852ypgvt0peqj8f5kx4x0s4zqcky4"],
  ["ow721", "orai1ase8wkkhczqdda83f0cd9lnuyvf47465j70hyk"],
];

async function setUpMarketPlaceEnviroment() {
  const client = new SimulateCosmWasmClient({
    chainId: "Oraichain",
    bech32Prefix: "orai",
    metering: true,
  });

  client.app.bank.setBalance(admin, [{ amount: "10000000000", denom: "orai" }]);

  const codeIdPromises = setUpList.map(([name, _address]) => {
    return client.upload(
      admin,
      readFileSync(resolve(__dirname, `../artifacts/${name}/${name}.wasm`))
    );
  });

  const codeIds = (await Promise.all(codeIdPromises)).map(
    (result) => result.codeId
  );

  const loadPromises = setUpList.map(([name, address], index) => {
    const data = JSON.parse(
      readFileSync(resolve(__dirname, `./testdata/${address}.json`)).toString()
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
  });
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

  market_721_impl = market721.contractAddress;
  market_1155_impl = market1155.contractAddress;

  return client;
}

describe("Test marketplace buy for", () => {
  let client;
  const ownerNFT = "orai14vcw5qk0tdvknpa38wz46js5g7vrvut8lk0lk6";
  const admin1155 = "orai1zsqaw40hcj4hk7r2g3xz864gda9vpq3ze9vpxc";
  const mintForAddress = "orai1w0emvx75v79x2rm0afcw7sn8hqt5f4fhtd3pw7";
  beforeAll(async () => {
    client = await setUpMarketPlaceEnviroment();
    client.app.bank.setBalance(ownerNFT, [
      { amount: "10000000000", denom: "orai" },
    ]);
    client.app.bank.setBalance(admin1155, [
      { amount: "10000000000", denom: "orai" },
    ]);
  }, 30000);

  it("Setup successfull", async () => {
    console.log("success");
  });

  it("Should return registry & adminlist from market_hub", async () => {
    const [registry, canExecute] = await Promise.all([
      client.queryContractSmart(market_hub, {
        registry: {},
      }),
      client.queryContractSmart(market_hub, {
        can_execute: {
          sender: "orai14vcw5qk0tdvknpa38wz46js5g7vrvut8lk0lk6",
        },
      }),
    ]);
    expect(canExecute).toBeTruthy();
    expect(registry).toBeTruthy();
  });

  it("should add the the new implement in market_hub", async () => {
    await client.execute(
      admin,
      market_hub,
      {
        update_implementation: {
          implementation: market_1155_impl,
        },
      },
      "auto"
    );

    await client.execute(
      admin,
      market_hub,
      {
        update_implementation: {
          implementation: market_721_impl,
        },
      },
      "auto"
    );
    const registry = await client.queryContractSmart(market_hub, {
      registry: {},
    });

    expect(registry.implementations.length).toBe(5);
  });

  it("should change minter in collection", async () => {
    await Promise.all([
      client.execute(ownerNFT, ow1155, {
        change_minter: { minter: market_1155_impl },
      }),
      client.execute(ownerNFT, ow721, {
        change_minter: { minter: market_721_impl },
      }),
    ]);

    const [minter1155] = await Promise.all([
      client.queryContractSmart(ow1155, {
        minter: {},
      }),
    ]);

    expect(minter1155).toEqual(market_1155_impl);
  });

  it("should mint for another user", async () => {
    await Promise.all([
      client.execute(
        admin1155,
        market_1155_impl,
        {
          mint_for_nft: {
            contract_addr: ow1155,
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
        market_721_impl,
        {
          mint_nft: {
            contract_addr: ow721,
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
      client.queryContractSmart(ow721, {
        nft_info: {
          token_id: "260900",
        },
      }),
      client.queryContractSmart(ow1155, {
        balance: {
          owner: mintForAddress,
          token_id: "260900",
        },
      }),
    ]);

    expect(token721Info.name).toEqual("TEST_TEST");
    expect(balance1155.balance).toEqual("2000");
  });

  it("should buy for another user", async () => {
    const buyForAddress = "orai149uraxsj6vqqugzth8drw8akjcuktkfl3tafue";

    const [offering1155, offering721] = await Promise.all([
      client.queryContractSmart(market_1155_storage, {
        msg: { get_offerings: {} },
      }),
      client.queryContractSmart(market_offering_storage, {
        offering: { get_offerings: {} },
      }),
    ]);

    const {
      id: latest_offering,
      token_id,
      per_price,
      seller,
    } = offering1155.pop();

    client.app.bank.setBalance(seller, [
      { amount: "10000000000", denom: "orai" },
    ]);

    // Approve
    await Promise.all([
      client.execute(
        seller,
        ow1155,
        {
          approve_all: { operator: market_1155_impl },
        },
        "auto"
      ),
    ]);

    await Promise.all([
      client.execute(
        admin1155,
        market_1155_impl,
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
    ]);

    const [sellerBalance, buyForBalance] = await Promise.all([
      client.queryContractSmart(ow1155, {
        balance: {
          owner: seller,
          token_id,
        },
      }),
      client.queryContractSmart(ow1155, {
        balance: {
          owner: buyForAddress,
          token_id,
        },
      }),
    ]);

    expect(sellerBalance.balance).toBe("0");
    expect(buyForBalance.balance).toBe("1");
  });
});

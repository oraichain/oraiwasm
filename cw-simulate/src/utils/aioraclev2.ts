import { coin } from "@cosmjs/amino";
import { SimulateCosmWasmClient } from "@terran-one/cw-simulate";

const admin = "admin_aioraclev2";
const client = new SimulateCosmWasmClient({
  chainId: "Oraichain-testnet",
  bech32Prefix: "orai",
});
const SERVICE_DEFAULT = "price";
const EXECUTOR_PUBKEY = "AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn";

export const aioraclev2 = async () => {
  console.log(1111111, executorsDemo());
  client.app.bank.setBalance(admin, [coin("10000000000", "orai")]);
  const serviceFeesContract = await serviceFees();
  const provideBridgeContract = await providerBridge(
    serviceFeesContract.contractAddress
  );
  const aioraclev2 = await client.deploy(
    admin,
    "../package/aioracle/aioracle_v2/artifacts/aioracle_v2.wasm",
    {
      owner: null,
      service_addr: provideBridgeContract.contractAddress,
      contract_fee: {
        denom: "orai",
        amount: "1",
      },
      executors: executorsDemo(),
    },
    "aioraclev2 label"
  );
  console.log("aioravlev2 deploy info", aioraclev2);

  // // // exec create stage executor txs
  let input: any = {
    request: {
      threshold: 1,
      service: SERVICE_DEFAULT,
      preference_executor_fee: { denom: "orai", amount: "1" },
    },
  };
  const funds = coin(10, "orai");
  const execCreateStage = await client.execute(
    admin,
    aioraclev2.contractAddress,
    input,
    "auto",
    null,
    [funds]
  );
  console.log("aioraclev2 exec create stage executor txs", execCreateStage);

  // // // exec register merkle root
  input = {
    register_merkle_root: {
      stage: 1,
      merkle_root:
        "4a2e27a2befb41a0655b8fe98d9c1a9f18ece280dc78b442734ead617e6bf3fc",
      executors: [executorSingle()],
      service: SERVICE_DEFAULT,
    },
  };
  const execRegisterMerkleRoot = await client.execute(
    admin,
    aioraclev2.contractAddress,
    input,
    "auto"
  );
  console.log("aioraclev2 exec register merkele root", execRegisterMerkleRoot);

  // // // query
  const queryConfig = await client.queryContractSmart(
    aioraclev2.contractAddress,
    {
      config: {},
    }
  );
  const queryExecutors = await client.queryContractSmart(
    aioraclev2.contractAddress,
    {
      get_executors: {},
    }
  );
  const queryBoundExecutorFee = await client.queryContractSmart(
    aioraclev2.contractAddress,
    {
      get_bound_executor_fee: {
        service: SERVICE_DEFAULT,
      },
    }
  );
  const queryGetParticipantFee = await client.queryContractSmart(
    aioraclev2.contractAddress,
    {
      get_participant_fee: {
        pubkey: EXECUTOR_PUBKEY,
        service: SERVICE_DEFAULT,
      },
    }
  );

  console.log(
    "aioraclev2 query config",
    queryConfig,
    "EXECUTORS",
    queryExecutors,
    "queryBoundExecutorFee",
    queryBoundExecutorFee,
    "queryGetParticipantFee",
    queryGetParticipantFee
  );
};

const providerBridge = async (serviceFeeContractAddr: string) => {
  const provideBridgeContract = await client.deploy(
    admin,
    "../package/aioracle/provider_bridge/artifacts/provider_bridge.wasm",
    {
      service: SERVICE_DEFAULT,
      service_contracts: {
        dsources: ["orai188efpndge9hqayll4cp9gzv0dw6rvj25e4slkp"],
        tcases: ["orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt"],
        oscript: "orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj",
      },
      service_fees_contract: serviceFeeContractAddr,
      bound_executor_fee: "1",
    },
    "provider bridge label"
  );
  console.log("provider bridge deploy info", provideBridgeContract);
  return provideBridgeContract;
};

const serviceFees = async () => {
  const serviceFeeContract = await client.deploy(
    admin,
    "../package/aioracle/aioracle_service_fees/artifacts/aioracle_service_fees.wasm",
    {},
    "service fee label"
  );
  console.log("service fee deploy info", serviceFeeContract);
  return serviceFeeContract;
};

const executorsDemo = (): any[] => {
  const pubKeys = [
    "Agq2Xl1IcoOt4IRhaA2pO7xq2SBGBfsQuopQnptmos1q",
    "Ahc1poKD9thmAX8dMgFCVKhpUjyVYHfB0q/XTwPuD/J/",
    "Ah11L/hsl9J9mXkH9xFzKQbw9F/wh0n6JaKitTzptYqR",
    "AiIhSld8auqXnAE2Hzcr5gBrmLaHxbFrIbZcpb3iG0Zz",
    "A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t",
    "A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA",
    "A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw",
    EXECUTOR_PUBKEY,
  ];
  return pubKeys.map((item) => executorSingle(item));
};

const executorSingle = (pubkey = EXECUTOR_PUBKEY) => {
  return Buffer.from(
    JSON.stringify({
      pubkey: pubkey,
      executing_power: 0,
      index: 1,
      is_active: true,
      left_block: null,
    })
  ).toString("base64");
};

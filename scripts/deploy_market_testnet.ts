import {
  DeliverTxResponse,
  MsgInstantiateContractEncodeObject,
  MsgStoreCodeEncodeObject,
  SigningCosmWasmClient,
} from "@cosmjs/cosmwasm-stargate";
import math from "@cosmjs/math";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { readFileSync } from "fs";
import { resolve } from "path";
import pako from "pako";
import long from "long";
import {
  MsgStoreCode,
  MsgInstantiateContract,
} from "cosmjs-types/cosmwasm/wasm/v1/tx.js";
import path from "path";
import { fileURLToPath } from "url";
import * as dotenv from "dotenv";
dotenv.config();

const __filename = fileURLToPath(import.meta.url);

const __dirname = path.dirname(__filename);

export function constructStoreCodeMsg(
  senderAddress: string,
  wasmCode: Uint8Array
): MsgStoreCodeEncodeObject {
  const compressed = pako.gzip(wasmCode, { level: 9 });
  return {
    typeUrl: "/cosmwasm.wasm.v1.MsgStoreCode",
    value: MsgStoreCode.fromPartial({
      sender: senderAddress,
      wasmByteCode: compressed,
    }) as any,
  };
}

export function extractAttribute(
  deliverTx: DeliverTxResponse,
  eventType: string,
  key: string
): string[] {
  return deliverTx.events
    .filter((event) => event.type === eventType)
    .map(
      (event) => event.attributes.filter((attr) => attr.key === key)[0].value
    );
}

export function constructInstatiateMsg(
  senderAddress: string,
  codeId: number,
  label: string,
  msg: any,
  admin: string
): MsgInstantiateContractEncodeObject {
  return {
    typeUrl: "/cosmwasm.wasm.v1.MsgInstantiateContract",
    value: MsgInstantiateContract.fromPartial({
      sender: senderAddress,
      codeId: long.fromString(new math.Uint53(codeId).toString()) as any,
      label: label,
      msg: new Uint8Array(Buffer.from(JSON.stringify(msg), "utf8")),
      funds: [],
      admin,
    }),
  };
}

const oraichain_testnet: any = {
  rpc: "https://testnet-rpc.orai.io",
  chainId: "Oraichain-testnet",
  denom: "orai",
  prefix: "orai",
  gasPrice: "0.024orai",
};

(async () => {
  let newRegistry: Record<string, string>;
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(
    process.env.MNEMONIC as string,
    {
      prefix: "orai",
    }
  );
  const [signer] = await wallet.getAccounts();
  const client = await SigningCosmWasmClient.connectWithSigner(
    oraichain_testnet.rpc,
    wallet,
    {
      prefix: oraichain_testnet.prefix,
      gasPrice: oraichain_testnet.gasPrice,
    }
  );

  const registry = JSON.parse(
    readFileSync(resolve(__dirname, "../utils/registry.json"), {
      encoding: "utf8",
    })
  );

  const initMsgs = JSON.parse(
    readFileSync(resolve(__dirname, "./initMsg.json"), {
      encoding: "utf8",
    })
  );

  const marketHub = registry["market_hub"];
  console.log(`Deploy market_hub`);
  const { contractAddress: governance } = await client
    .upload(
      signer.address,
      readFileSync(resolve(__dirname, `../tests/wasm/${marketHub}`)),
      "auto"
    )
    .then(({ codeId }) => {
      return client.instantiate(
        signer.address,
        codeId,
        initMsgs["market_hub"],
        "market_hub",
        "auto",
        { admin: signer.address }
      );
    });
  const batchSize = 3;
  const registryWithoutMarketHub = Object.entries(registry).filter(([name]) => {
    return name !== "market_hub";
  }) as [string, string][];

  let codeIds: string[] = [];

  for (let i = 0; i < registryWithoutMarketHub.length; i = i + batchSize) {
    console.log("Batching upload process: ", i / batchSize + 1, " time");
    const uploadMsgBatch = await Promise.all(
      registryWithoutMarketHub
        .slice(i, i + batchSize)
        .map(async ([_name, address]) => {
          return constructStoreCodeMsg(
            signer.address,
            readFileSync(resolve(__dirname, `../tests/wasm/${address}`))
          );
        })
    );
    const result = await client.signAndBroadcast(
      signer.address,
      uploadMsgBatch,
      "auto",
      ""
    );
    const codeIdExtract = extractAttribute(result, "store_code", "code_id");
    codeIds = [...codeIds, ...codeIdExtract];
  }

  const instantiateMsgs = registryWithoutMarketHub.map(
    ([name, _address], index) => {
      const msg = initMsgs[name];
      if (name !== "ow721" && name !== "ow1155" && name !== "ai_right") {
        msg.governance = governance;
      }
      console.log(name);
      console.log({ msg });

      return constructInstatiateMsg(
        signer.address,
        parseInt(codeIds[index]),
        name,
        msg,
        signer.address
      );
    }
  );

  const resultInstantiate = await client.signAndBroadcast(
    signer.address,
    instantiateMsgs,
    "auto",
    ""
  );
  const contractAddresses = extractAttribute(
    resultInstantiate,
    "instantiate",
    "_contract_address"
  );
  newRegistry = Object.fromEntries(
    registryWithoutMarketHub.map(([name, _], i) => [name, contractAddresses[i]])
  );
  newRegistry["market_hub"] = governance;
  return newRegistry;
})()
  .then((res) => {
    console.log("Deploy successfully!");
    console.log({ res });
  })
  .catch((err) => {
    console.log("Deploy fail!");
    console.log({ err });
  })
  .finally(() => {
    process.exit(1);
  });

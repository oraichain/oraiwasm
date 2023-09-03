import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import { resolve } from "path";
import * as dotenv from "dotenv";
import path from "path";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
dotenv.config();

const __filename = fileURLToPath(import.meta.url);

const __dirname = path.dirname(__filename);

const oraichain_testnet: any = {
  rpc: "https://testnet-rpc.orai.io",
  chainId: "Oraichain-testnet",
  denom: "orai",
  prefix: "orai",
  gasPrice: "0.024orai",
};

(async () => {
  const newRegistry = JSON.parse(
    readFileSync(resolve(__dirname, "./newRegistry.json"), { encoding: "utf8" })
  );
  const initMsg = JSON.parse(
    readFileSync(resolve(__dirname, "./initMsg.json"), { encoding: "utf8" })
  );
  console.log(initMsg);

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
  const { codeId: code721Id } = await client.upload(
    signer.address,
    readFileSync(
      resolve(
        __dirname,
        "../artifacts/market_implementation/market_implementation.wasm"
      )
    ),
    "auto",
    ""
  );
  const { codeId: code1155Id } = await client.upload(
    signer.address,
    readFileSync(
      resolve(
        __dirname,
        "../artifacts/market_1155_implementation/market_1155_implementation.wasm"
      )
    ),
    "auto",
    ""
  );
  console.log({ code721Id, code1155Id });
  const { contractAddress: market721Address } = await client.instantiate(
    signer.address,
    code721Id,
    initMsg["market_implementation"],
    "market_implementation",
    "auto",
    { admin: signer.address }
  );

  const { contractAddress: market1155Address } = await client.instantiate(
    signer.address,
    code1155Id,
    initMsg["market_1155_implementation"],
    "market_1155_implementation",
    "auto",
    { admin: signer.address }
  );

  console.log({ market721Address, market1155Address });
  // set up governance for market721 and market_1155_implementation
  await client.execute(
    signer.address,
    newRegistry["market_hub"],
    {
      update_implementation: {
        implementation: market721Address,
      },
    },
    "auto"
  );

  await client.execute(
    signer.address,
    newRegistry["market_hub"],
    {
      update_implementation: {
        implementation: market1155Address,
      },
    },
    "auto"
  );

  // update minter for ow721 and ow1155
  await client.execute(
    signer.address,
    newRegistry["ow721"],
    {
      change_minter: {
        minter: market721Address,
      },
    },
    "auto"
  );

  await client.execute(
    signer.address,
    newRegistry["ow1155"],
    {
      change_minter: {
        minter: market1155Address,
      },
    },
    "auto"
  );

  await client.execute(
    signer.address,
    newRegistry["market_hub"],
    {
      update_storages: {
        storages: Object.entries(newRegistry),
      },
    },
    "auto"
  );
})()
  .catch((error) => {
    console.log(error);
  })
  .finally(() => {
    process.exit(1);
  });

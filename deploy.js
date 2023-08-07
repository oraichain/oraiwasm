const { SigningCosmWasmClient } = require("@cosmjs/cosmwasm-stargate");
const { DirectSecp256k1HdWallet } = require("@cosmjs/proto-signing");
const {
  GasPrice,
  makeCosmoshubPath,
  calculateFee,
} = require("@cosmjs/stargate");
const path = require("path");
const fs = require("fs");

const config = {
  networks: {
    oraichain_testnet: {
      rpc: "https://testnet-rpc.orai.io",
      chainId: "Oraichain-testnet",
      denom: "orai",
      prefix: "orai",
      gasPrice: "0.024orai",
    },
    oraichain: {
      rpc: " https://rpc.orai.io",
      chainId: "Oraichain",
      denom: "orai",
      prefix: "orai",
      gasPrice: "0.024orai",
    },
  },
};

require("dotenv").config();

async function setUp() {
  const argv = process.argv;
  const package = argv[2];
  const wasmPath = path.resolve(__dirname, package);
  const initMsgPath = argv[3];
  const initMsgResolvePath = path.resolve(__dirname, initMsgPath);

  const network =
    process.env.NODE_ENV === "production"
      ? config.networks.oraichain
      : config.networks.oraichain_testnet;
  const mnemonic = process.env.MNEMONIC;
  const hdPaths = [makeCosmoshubPath(0)];
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: network.prefix,
    hdPaths,
  });
  const [signer] = await wallet.getAccounts();

  const cc = await SigningCosmWasmClient.connectWithSigner(
    network.rpc,
    wallet,
    {
      prefix: network.prefix,
      gasPrice: GasPrice.fromString(network.gasPrice),
    }
  );

  const wasm = fs.readFileSync(wasmPath);
  const uploadFee = calculateFee(0, GasPrice.fromString(network.gasPrice));
  console.log("=>uploadFee", uploadFee);
  const uploadResult = await cc.upload(signer.address, wasm, "auto");
  console.log("==>Codeid", uploadResult.codeId);

  const initMsg = JSON.parse(fs.readFileSync(initMsgResolvePath));

  const response = await cc.instantiate(
    signer.address,
    uploadResult.codeId,
    initMsg,
    "orai_market_1155_implementation",
    "auto"
  );

  console.log(response.contractAddress);
}

setUp()
  .then((_result) => {
    process.exit(1);
  })
  .catch((err) => {
    console.log(err);
    process.exit(1);
  });

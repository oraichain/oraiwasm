import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { AccountData, DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";

export enum NETWORK {
  MAINNET = "mainnet",
  TESTNET = "testnet",
}
export const NETWORK_CONFIG = {
  [NETWORK.TESTNET]: {
    rpc: "https://testnet-rpc.orai.io",
    chainId: "Oraichain-testnet",
    denom: "orai",
    prefix: "orai",
    gasPrice: "0.024orai",
  },
  [NETWORK.MAINNET]: {
    rpc: "https://rpc.orai.io",
    chainId: "Oraichain",
    denom: "orai",
    prefix: "orai",
    gasPrice: "0.024orai",
  },
};

export async function createClient(
  mnemonic: string,
  network: NETWORK = NETWORK.TESTNET
): Promise<[SigningCosmWasmClient, AccountData]> {
  const network_config = NETWORK_CONFIG[network];
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: "orai",
  });
  const [signer] = await wallet.getAccounts();
  const client = await SigningCosmWasmClient.connectWithSigner(
    network_config.rpc,
    wallet,
    {
      prefix: network_config.prefix,
      gasPrice: GasPrice.fromString(network_config.gasPrice),
    }
  );

  return [client, signer];
}

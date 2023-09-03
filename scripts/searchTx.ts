import { decodeTxRaw } from "@cosmjs/proto-signing";
import { createClient } from "./client.ts";
import * as dotenv from "dotenv";

dotenv.config();

(async () => {
  const [client, signer] = await createClient(process.env.MNEMONIC as any);

  const txs = await client.searchTx({
    tags: [
      {
        key: "transfer.recipient",
        value: "orai1w0emvx75v79x2rm0afcw7sn8hqt5f4fhtd3pw7",
      },
      {
        key: "transfer.sender",
        value: "orai1mw5ua2gttkw0e4whh0z59zpq0eg3e9gtse3uns",
      },
    ],
  });

  console.log(decodeTxRaw(txs[1].tx).body.messages);
  // console.log(txs);
})();

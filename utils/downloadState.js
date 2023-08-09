const { appendFileSync } = require("fs");

const downloadState = async (contractAddress, writeCallback, limit = 200) => {
  let nextKey;
  while (true) {
    const url = new URL(
      `https://lcd.orai.io/cosmwasm/wasm/v1/contract/${contractAddress}/state`
    );
    url.searchParams.append("pagination.limit", limit);
    if (nextKey) {
      url.searchParams.append("pagination.key", nextKey);
    }
    try {
      const { models, pagination } = await fetch(url.toString(), {}).then(
        (res) => res.json()
      );
      await writeCallback(models);
      if (!(nextKey = pagination.next_key)) break;
    } catch (ex) {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
};

// (async () => {
//   const contractAddress = "orai14tqq093nu88tzs7ryyslr78sm3tzrmnpem6fak";
//   downloadState(
//     contractAddress,
//     (chunks) => {
//       appendFileSync(
//         `${contractAddress}.state.csv`,
//         chunks
//           .map(
//             ({ key, value }) =>
//               `${Buffer.from(key, "hex").toString("base64")},${value}`
//           )
//           .join("\n") + "\n"
//       );
//     },
//     1000
//   );
// })();

module.exports = downloadState;

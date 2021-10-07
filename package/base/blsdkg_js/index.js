const fs = require("fs");
const path = require("path");
const YAML = require("yaml");
const dotenv = require("dotenv");
const Cosmos = require("@oraichain/cosmosjs").default;
const blsdkgJs = require("./pkg/blsdkg_js");

const { queryWasm, executeWasm, encrypt, decrypt, delay, convertOffset } = require("./utils");

const env = dotenv.config({
  path: `.env${process.env.NODE_ENV ? "." + process.env.NODE_ENV : ""}`,
}).parsed;

const config = YAML.parse(fs.readFileSync(path.join(__dirname, "config.yaml")).toString());
const message = Cosmos.message;
const cosmos = new Cosmos(config.url, config.chain_id);
cosmos.setBech32MainPrefix(config.denom || "orai");
const childKey = cosmos.getChildKey(env.MNEMONIC);

const address = cosmos.getAddress(childKey);

const run = async () => {
  const { status, threshold, total } = await queryWasm(cosmos, config.contract, {
    contract_info: {},
  });

  switch (status) {
    case "WaitForDealer":
      return await processDealer(threshold, total);
    case "WaitForRow":
    case "WaitForRequest":
      const currentMember = await getMember(address);
      if (!currentMember) {
        return console.log("we are not in the group");
      }
      const skShare = await getSkShare(currentMember);
      if (!skShare) {
        return console.log("row share is invalid");
      }
      if (status === "WaitForRow") {
        if (currentMember.shared_row) {
          return console.log("we are done row sharing");
        }
        return await processRow(skShare);
      }
      // default process each request
      return await processRequest(skShare);
    default:
      return console.log("Unknown status", status);
  }
};

// TODO: assume members are small, for big one should get 10 by 10
const getMembers = async (total) => {
  let offset = convertOffset("");
  let members = [];
  do {
    const tempMembers = await queryWasm(cosmos, config.contract, {
      get_members: {
        offset: offset,
      },
    });
    members = members.concat(tempMembers);
    offset = convertOffset(members[members.length - 1].address);
  } while (members.length < total);
  members = [...new Set(members)];
  return members;
};

const getMember = async (address) => {
  const member = await queryWasm(cosmos, config.contract, {
    get_member: {
      address
    }
  });

  return member.deleted ? undefined : member;
};

// TODO: handle get batch dealers when list is large
const getDealers = async (total) => {
  let offset = convertOffset("");
  let dealers = [];
  do {
    const tempDealers = await queryWasm(cosmos, config.contract, {
      get_dealers: {
        offset,
      },
    });
    dealers = dealers.concat(tempDealers);
    offset = convertOffset(dealers[dealers.length - 1].address);
  } while (dealers.length < total);
  dealers = [...new Set(dealers)];
  return dealers;
};

const processDealer = async (threshold, total) => {
  console.log("process dealer share");
  const bibars = blsdkgJs.generate_bivars(threshold, total);

  const commits = bibars.get_commits().map(Buffer.from);
  const rows = bibars.get_rows().map(Buffer.from);

  const members = await getMembers(total);
  // then sort members by index for sure to encrypt by their public key
  members.sort((a, b) => a.index - b.index);

  // check wherther we has done sharing ?
  const currentMember = members.find((m) => !m.deleted && m.address === address);

  if (!currentMember) {
    return console.log("we are not in the group");
  }

  if (currentMember.shared_dealer) {
    return console.log("we are done dealer sharing");
  }

  commits[0] = commits[0].toString("base64");
  for (let i = 0; i < rows.length; ++i) {
    // no need to check pubkey the same as address, they may use their desired keypair, bydefault it is the private key
    // remember commit[0] is the sum commit
    rows[i] = encrypt(Buffer.from(members[i].pubkey, "base64"), childKey.privateKey, commits[i + 1], rows[i]).toString("base64");
    commits[i + 1] = commits[i + 1].toString("base64");
  }

  // console.log(commits, rows);

  // finaly share the dealer
  const response = await executeWasm(cosmos, childKey, config.contract, {
    share_dealer: {
      share: {
        commits,
        rows,
      },
    },
  });

  // log response then return
  console.log(response);
};

const getSkShare = async (currentMember) => {
  const dealers = await getDealers();

  const commits = [];
  const rows = [];
  for (const dealer of dealers) {
    const encryptedRow = Buffer.from(dealer.shared_dealer.rows[currentMember.index], "base64");
    const dealerPubkey = Buffer.from(dealer.pubkey, "base64");
    const commit = Buffer.from(dealer.shared_dealer.commits[currentMember.index + 1], "base64");
    const row = decrypt(childKey.privateKey, dealerPubkey, commit, encryptedRow);
    commits.push(commit);
    rows.push(row);
  }

  const skShare = blsdkgJs.get_sk_share(rows, commits);

  return skShare;
};

const processRow = async (skShare) => {
  console.log("process row share");
  // we update public key share for smart contract to verify and keeps this skShare to sign message for each round
  const pkShare = Buffer.from(skShare.get_pk()).toString("base64");
  // finaly share the dealer
  const response = await executeWasm(cosmos, childKey, config.contract, {
    share_row: {
      share: {
        pk_share: pkShare,
      },
    },
  });

  console.log(response);
};

const processRequest = async (skShare) => {
  console.log("process request");

  // get current handling round
  const roundInfo = await queryWasm(cosmos, config.contract, {
    current_handling: {},
  });

  if (!roundInfo) {
    return console.log("there is no round");
  }

  if (roundInfo.combined_sig) {
    return console.log("Round has been done with randomness", roundInfo.randomness);
  }

  // otherwise add the sig contribution from skShare
  const sig = skShare.sign_g2(Buffer.from(roundInfo.input, "base64"), BigInt(roundInfo.round));

  const share = {
    sig: Buffer.from(sig).toString("base64"),
    round: roundInfo.round,
  };

  // console.log(address, shareSig);

  // share the signature, more gas because the verify operation, especially the last one
  const response = await executeWasm(
    cosmos,
    childKey,
    config.contract,
    {
      share_sig: {
        share,
      },
    },
    { gas: 20000000 }
  );
  console.log(response);
};

// run interval, default is 5000ms block confirmed
const runInterval = async (interval = 5000) => {
  while (true) {
    await run();
    await delay(interval);
  }
};

// for testing purpose, input is base64 to be pass as Buffer
const requestRandom = async (input) => {
  const response = await executeWasm(cosmos, childKey, config.contract, {
    request_random: {
      input,
    },
  });
  console.log(response);
};

const ping = async () => {

  // collect info about round and round jump, ok to ping => ping
  const round = await queryWasm(cosmos, config.ping_contract, {
    get_round: address
  });
  // valid case
  if ((round.current_height - round.round_info.height >= round.round_jump) || round.round_info.height === 0) {
    console.log("ready to ping");
    const response = await executeWasm(
      cosmos,
      childKey,
      config.ping_contract,
      {
        ping: {},
      },
      { gas: 2000000 }
    );
    console.log(response);
  }
}

// run interval to ping, default is 5000ms block confirmed
const addPing = async (interval = 5000) => {
  while (true) {
    await ping();
    await delay(interval);
  }
};

runInterval(config.interval);
addPing(env.PING_INTERVAL);

// TODO: add try catch and improve logs
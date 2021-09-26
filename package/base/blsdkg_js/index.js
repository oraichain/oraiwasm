const fs = require('fs');
const path = require('path');
const YAML = require('yaml');
const dotenv = require('dotenv');
const Cosmos = require('@oraichain/cosmosjs').default;
const blsdkgJs = require('./pkg/blsdkg_js');

const { queryWasm, executeWasm, encrypt, decrypt } = require('./utils');

const env = dotenv.config({
  path: `.env${process.env.NODE_ENV ? '.' + process.env.NODE_ENV : ''}`
}).parsed;

const config = YAML.parse(
  fs.readFileSync(path.join(__dirname, 'config.yaml')).toString()
);
const message = Cosmos.message;
const cosmos = new Cosmos(config.url, config.chain_id);
cosmos.setBech32MainPrefix(config.denom || 'orai');
const childKey = cosmos.getChildKey(env.MNEMONIC);

const address = cosmos.getAddress(childKey);

const run = async () => {
  const { status, threshold, total } = await queryWasm(
    cosmos,
    config.contract,
    {
      contract_info: {}
    }
  );

  switch (status) {
    case 'WaitForDealer':
      return processDealer(threshold, total);
    case 'WaitForRow':
      return processRow(total);
    case 'WaitForRequest':
      return processRequest();
    default:
      return console.log('Unknown status', status);
  }
};

// TODO: assume members are small, for big one should get 10 by 10
const getMembers = async (limit) => {
  const members = await queryWasm(cosmos, config.contract, {
    get_members: {
      limit
    }
  });

  return members;
};

const getDealers = async () => {
  const dealers = await queryWasm(cosmos, config.contract, {
    get_dealers: {}
  });

  return dealers;
};

const processDealer = async (threshold, total) => {
  const bibars = blsdkgJs.generate_bivars(threshold, total);

  const commits = bibars.get_commits().map(Buffer.from);
  const rows = bibars.get_rows().map(Buffer.from);

  const members = await getMembers(total);
  // then sort members by index for sure to encrypt by their public key
  members.sort((a, b) => a.index - b.index);

  // check wherther we has done sharing ?
  const currentMember = members.find(
    (m) => !m.deleted && m.address === address
  );

  if (!currentMember) {
    return console.log('we are not in the group');
  }

  if (currentMember.shared_dealer) {
    return console.log('we are done dealer sharing');
  }

  commits[0] = commits[0].toString('base64');
  for (let i = 0; i < rows.length; ++i) {
    // no need to check pubkey the same as address, they may use their desired keypair, bydefault it is the private key
    // remember commit[0] is the sum commit
    rows[i] = encrypt(
      Buffer.from(members[i].pubkey, 'base64'),
      childKey.privateKey,
      commits[i + 1],
      rows[i]
    ).toString('base64');
    commits[i + 1] = commits[i + 1].toString('base64');
  }

  // console.log(commits, rows);

  // finaly share the dealer
  const response = await executeWasm(cosmos, childKey, config.contract, {
    share_dealer: {
      share: {
        commits,
        rows
      }
    }
  });

  // log response then return
  console.log(response);
};

const processRow = async (total) => {
  const dealers = await getDealers();
  // get all rows and commits share from dealers for this member
  console.log(dealers);
};

run();

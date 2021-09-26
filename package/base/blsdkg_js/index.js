const fs = require('fs');
const path = require('path');
const YAML = require('yaml');
const dotenv = require('dotenv');
const Cosmos = require('@oraichain/cosmosjs').default;
const blsdkgJs = require('./pkg/blsdkg_js');

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

console.log(cosmos.getAddress(childKey));

const msg = 'hello';
const sk = Buffer.from(
  'V26g2BlNdF1/uRlqEQmsvIw2tkjYaiB5ey6L+6xrnnE=',
  'base64'
);
const sig =
  'ptNS57WXJoCz8HFyG6EyA73WRkOOlKnf/aB7lJ74K3XH8ZENQI+/3lJqFOLNH8DEBExk0I9WzUWO0hrTB8nakkLDrR92+Wz5Sxl5dFEusujuHOU9cpHWyu3GmflBKKkC';
const ret = Buffer.from(blsdkgJs.sign(sk, msg));
console.log(ret.toString('base64') === sig);

const bibars = blsdkgJs.generate_bivars(2, 5);

const commits = bibars.get_commits();
const rows = bibars.get_rows();

console.log(
  'commits',
  commits.map((commit) => Buffer.from(commit).toString('base64'))
);

console.log(
  'rows',
  rows.map((row) => Buffer.from(row).toString('base64'))
);

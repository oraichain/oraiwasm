const level = require('level');
const {
  MerkleProofTree,
  sha256,
  verifyHexProof
} = require('./ow20-merkle-airdrop');
const data = require('../testdata/airdrop_stage_2_list.json');

const db = level('merkle-proof', {
  keyEncoding: 'binary',
  valueEncoding: 'binary'
});

(async () => {
  const values = data.map(JSON.stringify);
  const leaves = values.map(sha256);

  const tree = new MerkleProofTree(leaves);
  await db.put(tree.getRoot(), Buffer.concat(leaves));
  await Promise.all(values.map((value, i) => db.put(leaves[i], value)));

  console.log('save data in', tree.getHexRoot());
})();

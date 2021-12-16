const fs = require('fs');
const path = require('path');
const {
  MerkleProofTree,
  sha256,
  verifyHexProof
} = require('./merkle-proof-tree');

const testdataInd = process.argv[2] || '1';
const data = require(`../testdata/airdrop_stage_${testdataInd}_list.json`);

const tree = new MerkleProofTree(data.map(JSON.stringify).map(sha256));

const proofItem = {
  address: data[0].address,
  data: JSON.stringify(data[0].data),
  root: tree.getHexRoot()
};
const hexLeaf = sha256(JSON.stringify(data[0])).toString('hex');
proofItem.proofs = tree.getHexProof(hexLeaf);

console.log('proofItem', proofItem);
fs.writeFileSync(
  path.resolve(
    __dirname,
    '..',
    'testdata',
    `airdrop_stage_${testdataInd}_test_data.json`
  ),
  JSON.stringify(proofItem, null, 2)
);

const verified = verifyHexProof(hexLeaf, proofItem.proofs, proofItem.root);
console.log('verified', verified);

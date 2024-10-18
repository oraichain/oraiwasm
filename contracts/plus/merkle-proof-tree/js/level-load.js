const level = require('level');
const {
  MerkleProofTree,
  sha256,
  verifyHexProof
} = require('./merkle-proof-tree');

const db = level('merkle-proof', {
  keyEncoding: 'binary',
  valueEncoding: 'binary'
});

(async () => {
  // load data
  const rootHex =
    process.argv[2] ||
    'bca58c8e5c526508fb76660b7891354cf09c464f0bc3941e95ba46a5b048d757';
  const keys = await db.get(Buffer.from(rootHex, 'hex'));

  const leaves = [];
  for (let i = 0; i < keys.length; i += 32) {
    leaves.push(keys.slice(i, i + 32));
  }
  const tree = new MerkleProofTree(leaves);

  const hexLeaf = sha256(
    JSON.stringify({
      address: 'wasm1k9hwzxs889jpvd7env8z49gad3a3633vg350tq',
      data: {
        amount: '666666666'
      }
    })
  ).toString('hex');

  const proofs = tree.getHexProof(hexLeaf);

  const data = await Promise.all(
    leaves.map((leaf) => db.get(leaf).then(JSON.parse))
  );

  const verified = verifyHexProof(hexLeaf, proofs, tree.getHexRoot());
  console.log('verified', verified);

  console.log(data);
})();

const anchor = require('@project-serum/anchor');
const {
    MerkleProofTree,
    sha256,
} = require('./merkle-proof-tree');

/// tree format
// const data = [
//     {
//         orai_pub: "AmgbPq+M9/qELxUEceBWrZ+Hbn1FoVAH6zZpWW5UVlWU",
//         sol_pub: "A24GCFcycHGnVQZ2ijS8415JuuD8yHjS1jDiueUpABvP", // must in base58
//     },
//     {
//         orai_pub: "A2JyjvCWNpj83BR+UbXkBSbSp7nW71V4hg4YlhqxZRJA",
//         sol_pub: "A24GCFcycHGnVQZ2ijS8415JuuD8yHjS1jDiueUpABvP",
//     }
// ]

const getBufferForHash = ({ orai_pub, sol_pub }) => {
    const pubkey = new anchor.web3.PublicKey(sol_pub);
    return Buffer.concat([pubkey.toBuffer(), Buffer.from(orai_pub, 'base64')]);
};

const getTree = (data) => {
    return new MerkleProofTree(data.map(getBufferForHash).map(sha256));
}

module.exports = {
    getTree,
    getBufferForHash,
}
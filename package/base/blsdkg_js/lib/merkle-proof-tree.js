const { MerkleTree } = require('merkletreejs');
const crypto = require('crypto');

const sha256 = (data) => crypto.createHash('sha256').update(data).digest();
const verifyHexProof = (hexLeaf, hexProof, hexRoot) => {
    const leaf = Buffer.from(hexLeaf, 'hex');
    const proof = hexProof.map((hex) => Buffer.from(hex, 'hex'));
    return verifyProof(leaf, proof, Buffer.from(hexRoot, 'hex'));
};

const verifyProof = (leaf, proof, root) => {
    const hashBuf = proof.reduce(
        (hashBuf, proofBuf) =>
            sha256(Buffer.concat([hashBuf, proofBuf].sort(Buffer.compare))),
        leaf
    );

    return Buffer.compare(root, hashBuf) === 0;
};

class MerkleProofTree extends MerkleTree {
    constructor(leaves) {
        super(leaves, undefined, { sort: true });
    }

    getHexProof(leaf, index) {
        return super.getHexProof(leaf, index).map((x) => x.substring(2));
    }

    getHexRoot() {
        return super.getHexRoot().substring(2);
    }

    getHexLeaves() {
        return super.getHexLeaves().map((x) => x.substring(2));
    }
}

module.exports = { sha256, verifyHexProof, MerkleProofTree, verifyProof };

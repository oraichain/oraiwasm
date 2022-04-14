const anchor = require('@project-serum/anchor');
const { getBufferForHash, tree, getTree } = require('./lib/form-tree');
const {
    sha256,
    verifyProof
} = require('./lib/merkle-proof-tree');
const idl = require('../../../../anchor-vrf/target/idl/vrf.json');
const { SOLANA_CONFIG } = require('./constants');
const { queryWasm } = require('./utils');

anchor.setProvider(anchor.Provider.local());

const verifyAndGetProof = (tree, oraiPub, solAddr) => {
    const leaf = sha256(getBufferForHash({ orai_pub: oraiPub.toString('base64'), sol_pub: solAddr }));
    const proof = tree.getProof(leaf).map(({ data }) => data);

    const verified = verifyProof(leaf, proof, tree.getRoot());
    console.log('is the signer verified', verified);

    return Buffer.concat(proof);
}

const addRandomness = async (cosmos, roundNum, input, randomness, oraiPub, requester) => {
    // TODO: Need to loop until get all full members
    const data = await queryWasm(cosmos, SOLANA_CONFIG.ORAICHAIN_TESTNET_SOLLIST_ADDR, {
        get_members: {}
    });
    const tree = getTree(data);

    // const programId = anchor.web3.Keypair.fromSecretKey(
    //     Buffer.from(keypair)
    // ).publicKey;

    const programId = SOLANA_CONFIG.LOCALNET_PROGRAM_ID;

    const program = new anchor.Program(idl, programId);
    const payer = anchor.getProvider().wallet;
    let lamportsBalance = await anchor
        .getProvider()
        .connection.getBalance(payer.publicKey);

    if (lamportsBalance < 1000 * anchor.web3.LAMPORTS_PER_SOL) {
        const signature = await anchor
            .getProvider()
            .connection.requestAirdrop(
                payer.publicKey,
                1000 * anchor.web3.LAMPORTS_PER_SOL
            );
        await anchor.getProvider().connection.confirmTransaction(signature);
        lamportsBalance = await anchor
            .getProvider()
            .connection.getBalance(payer.publicKey);
    }

    const round = new anchor.BN(roundNum);
    const randomnessBuff = Buffer.from(randomness, 'base64');

    const seed = sha256(
        Buffer.concat([program.programId.toBuffer(), tree.getRoot(), payer.publicKey.toBuffer(), Buffer.from(requester, 'base64'), Buffer.from(input, 'base64')])
    );
    const sigAccount = anchor.web3.Keypair.fromSeed(
        seed
    );

    // deserialize to sharedSignature
    const sigRecord = await program.account.randomnessData.fetchNullable(
        sigAccount.publicKey
    );
    if (!sigRecord) {
        const proofBuffer = verifyAndGetProof(tree, oraiPub, payer.publicKey.toString('base64'));
        return program.rpc.addRandomness(
            Buffer.from(oraiPub, 'base64'),
            tree.getRoot(),
            proofBuffer,
            round,
            randomnessBuff,
            {
                accounts: {
                    randomnessData: sigAccount.publicKey,
                    user: payer.publicKey,
                    systemProgram: anchor.web3.SystemProgram.programId
                },
                signers: [sigAccount]
            }
        );
    }
}

module.exports = { addRandomness };
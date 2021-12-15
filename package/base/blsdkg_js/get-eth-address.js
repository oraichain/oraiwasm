const publicKeyToAddress = require('ethereum-public-key-to-address');
const Cosmos = require('@oraichain/cosmosjs').default;
const {
    queryWasm,
    executeWasm,
    encrypt,
    decrypt,
    delay,
    convertOffset,
    env,
    signSignature
} = require('./utils');

require('dotenv').config({ path: `${process.env.NODE_ENV ? process.env.NODE_ENV : ""}.env` }).parsed;
const cosmos = new Cosmos(process.env.URL, process.env.CHAIN_ID);
cosmos.setBech32MainPrefix('orai');

// TODO: assume members are small, for big one should get 10 by 10
const getMembers = async (total) => {
    let offset = convertOffset('');
    let members = [];
    do {
        const tempMembers = await queryWasm(cosmos, process.env.CONTRACT, {
            get_members: {
                offset,
                limit: 5
            }
        });
        if (!tempMembers || tempMembers.code || tempMembers.length === 0) continue;
        members = members.concat(tempMembers);
        offset = convertOffset(members[members.length - 1].address);
        members = members.filter(
            (v, i, a) => a.findIndex((t) => t.index === v.index) === i
        );
        // if no more data, we also need to break
        // if (oldOffset === offset) break;
        // oldOffset = offset;
    } while (members.length < total);
    return members;
};

const getAddresses = async () => {

    const { status, threshold, total, dealer } = await queryWasm(
        cosmos,
        process.env.CONTRACT,
        {
            contract_info: {}
        }
    );

    let members = await getMembers(total);
    members = members.map(member => ({ pubkey: member.pubkey, address: publicKeyToAddress(Buffer.from(member.pubkey, 'base64')) }))
    console.log("members: ", members);
}

getAddresses();

// URL=http://lcd.orai.io CHAIN_ID=Oraichain CONTRACT=orai15lv4hxxqew2jhfayfmad7y40zfr8zmgfulqdxj node get-eth-address.js
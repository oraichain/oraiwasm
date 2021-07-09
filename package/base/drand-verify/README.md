# VRF

Create a small blockchain on a random chain smart contract

Has n rounds, start at 0. Smart contract contains an aggregated public key of all participants (priv-pub pairs generated from BLS signature scheme)

## Phase 1

Has one participant, also executor, only random chain smart contract

### Step 1

Interval k minutes, participant generates new signature (sign on message including: # round & aggregated signature of the previous round). If round 0 => no previous signature.

### Step 2

Update signature onto the random chain smart contract, update round & new signature. Randomness = hash(new signature)

### Step 3

Use randomess value as seed for existing random functions

Example query: https://lcd.testnet.orai.io/wasm/v1beta1/contract/orai1j9a0uu4qth30xuud3wg7eamd7vvs2nxnnupqr2/smart/eyJsYXRlc3QiOnt9fQ==

## Phase 2

Has a random request smart contract

### Step 1

When a user creates a random request to a random request smart contract (similar to the aioracle smart contract, different from random chain smart contract) => participants run websocket clients, listen to the request

### Step 2

Participants generate a signature (sign on message including: # round & aggregated signature of the previous round). If round 0 => no previous signature, and create a report including the signature & store onto the smart contract

### Step 3

An executor (can be anyone), listens to the report updates of the random request smart contract, collect enough signatures then aggregate them into one signature (using BLS signature scheme to aggregate). The threshold of the signatures depend on the rule we set (but min 50% because BLS scheme needs at least 50% of the signatures to verify)

### Step 4

Executor sends the aggregated signature onto the random chain smart contract, smart contract verifies using the aggregated public key & hash the aggregated signature to get a randomness hash value, update round and signature.

### Step 5

Randomness value is used as a seed for existing random functions.

Example query: https://lcd.testnet.orai.io/wasm/v1beta1/contract/orai1j9a0uu4qth30xuud3wg7eamd7vvs2nxnnupqr2/smart/eyJsYXRlc3QiOnt9fQ==

## Phase 3

Add reward & bounty for the participants & executor


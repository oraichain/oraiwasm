## Distributed verifiable randomness function sub-network

- Verifiable, unpredictable and unbiased random numbers as a service using smart contract running on Oraichain network.

- Randomness serves a vital role in many aspect of a blockchain world. Voting systems, financial services, and the most widely used is within the field of NFT games.

#### Features of good randomness

- Unpredictable: you can not predict the next number to come out of the generator, because the output derived from cryptographic functions using private seeds.

- Publicly-verifiable: anyone can verify that a random number is calculated correctly with provided proofs, a combined signature and a combined public key.

- Bias-resistant: no one can behave disonestly to lead the generator toward their advantage, because each shared part can be verified on-chain.

- Decentralized: a set of independent parties produces random numbers, by commiting all their public keys in init phrase and broad cast their signatures in each round.

- Available: the system always be able to provide random numbers, by running on blockchain.

- Fault Tolerance: the system can continue to provide random numbers if some nodes are down or have any failures, thanks to Threshold Signature Scheme.

#### How sub-network works

A sub-network is made up of a set number of nodes running on Oraichain smart contract protocol. Before generating random numbers, they all agree on a threshold parameter that is set in a smart contract. Later, a sub set of nodes will act like dealer to share all secret rows encrypted by corresponding public keys and public commits on blockchain. Each node will collect all the secret rows and decrypted with their private key to combine into a secret share that no one knowns, and publish the public share on smart contract so that anyone can verify their later works.

To produce a random number, a random request is required with a given input. Each node creates a signature by signing the given input and the round number together, then publish their signature to the rest of the sub-network. The smart contract protocol waits and collects these signatures until it has enough signatures to match the threshold parameter, to produce the final signature.

The final signature is a regular [Boneh–Lynn–Shacham](https://en.wikipedia.org/wiki/BLS_digital_signature) signature that can be verified against by the rest of the network. If that signature is correct, then the randomness is simple the hash of that signature, currently the sha3-256.

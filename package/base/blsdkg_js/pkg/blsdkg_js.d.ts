/* tslint:disable */
/* eslint-disable */
/**
* @param {Uint8Array} sk
* @param {Uint8Array} msg
* @returns {Uint8Array | undefined}
*/
export function sign(sk: Uint8Array, msg: Uint8Array): Uint8Array | undefined;
/**
* @param {(Uint8Array)[]} rows
* @param {(Uint8Array)[]} commits
* @returns {KeyShare | undefined}
*/
export function get_sk_share(rows: (Uint8Array)[], commits: (Uint8Array)[]): KeyShare | undefined;
/**
* @param {number} threshold
* @param {number} total_nodes
* @returns {Share}
*/
export function generate_bivars(threshold: number, total_nodes: number): Share;
/**
*/
export class KeyShare {
  free(): void;
/**
* @returns {Uint8Array}
*/
  get_pk(): Uint8Array;
/**
* @param {Uint8Array} input
* @param {BigInt} round
* @returns {Uint8Array}
*/
  sign_g2(input: Uint8Array, round: BigInt): Uint8Array;
}
/**
*/
export class Share {
  free(): void;
/**
* @returns {(Uint8Array)[]}
*/
  get_commits(): (Uint8Array)[];
/**
* @returns {(Uint8Array)[]}
*/
  get_rows(): (Uint8Array)[];
}

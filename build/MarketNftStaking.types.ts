export type Addr = string;
export interface InstantiateMsg {
  admin?: Addr | null;
  nft_1155_contract_addr_whitelist: Addr[];
  nft_721_contract_addr_whitelist: Addr[];
  verifier_pubkey_base64: string;
}
export type ExecuteMsg = {
  update_contract_info: UpdateContractInfoMsg;
} | {
  create_collection_pool: CreateCollectionPoolMsg;
} | {
  update_collection_pool: UpdateCollectionPoolMsg;
} | {
  receive_nft: Cw721ReceiveMsg;
} | {
  receive: Cw1155ReceiveMsg;
} | {
  withdraw: {
    collection_id: string;
    withdraw_nft_ids: string[];
    withdraw_rewards: boolean;
  };
} | {
  claim: {
    collection_id: string;
  };
} | {
  reset_earned_rewards: {
    collection_id: string;
    staker: Addr;
  };
};
export type Uint128 = string;
export type Binary = string;
export interface UpdateContractInfoMsg {
  admin?: Addr | null;
  nft_1155_contract_addr_whitelist?: Addr[] | null;
  nft_721_contract_addr_whitelist?: Addr[] | null;
  verifier_pubkey_base64?: string | null;
}
export interface CreateCollectionPoolMsg {
  collection_id: string;
  expired_after?: number | null;
  reward_per_block: Uint128;
}
export interface UpdateCollectionPoolMsg {
  collection_id: string;
  reward_per_block?: Uint128 | null;
}
export interface Cw721ReceiveMsg {
  msg?: Binary | null;
  sender: Addr;
  token_id: string;
}
export interface Cw1155ReceiveMsg {
  amount: Uint128;
  from?: string | null;
  msg: Binary;
  operator: string;
  token_id: string;
}
export type QueryMsg = {
  get_contract_info: {};
} | {
  get_collection_pool_info: {
    collection_id: string;
  };
} | {
  get_collection_pool_infos: {
    limit?: number | null;
    offset?: number | null;
    order?: number | null;
  };
} | {
  get_unique_collection_staker_info: {
    collection_id: string;
    staker_addr: Addr;
  };
} | {
  get_collection_staker_info_by_collection: {
    collection_id: string;
    limit?: number | null;
    offset?: number | null;
    order?: number | null;
  };
} | {
  get_collection_staker_info_by_staker: {
    limit?: number | null;
    offset?: number | null;
    order?: number | null;
    staker_addr: Addr;
  };
};
export type NullableCollectionPoolInfo = CollectionPoolInfo | null;
export interface CollectionPoolInfo {
  acc_per_share: Uint128;
  collection_id: string;
  expired_block?: number | null;
  last_reward_block: number;
  reward_per_block: Uint128;
  total_nfts: Uint128;
}
export type ArrayOfCollectionPoolInfo = CollectionPoolInfo[];
export type ContractType = "V721" | "V1155";
export type ArrayOfCollectionStakerInfo = CollectionStakerInfo[];
export interface CollectionStakerInfo {
  collection_id: string;
  id?: number | null;
  pending: Uint128;
  reward_debt: Uint128;
  staked_tokens: CollectionStakedTokenInfo[];
  staker_addr: Addr;
  total_earned: Uint128;
  total_staked: Uint128;
}
export interface CollectionStakedTokenInfo {
  amount: Uint128;
  contract_addr: Addr;
  contract_type: ContractType;
  token_id: string;
}
export interface ContractInfo {
  admin: Addr;
  nft_1155_contract_addr_whitelist: Addr[];
  nft_721_contract_addr_whitelist: Addr[];
  verifier_pubkey_base64: string;
}
export type NullableCollectionStakerInfo = CollectionStakerInfo | null;
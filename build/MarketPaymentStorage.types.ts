export type Addr = string;
export interface InstantiateMsg {
  governance: Addr;
}
export type ExecuteMsg = {
  msg: PaymentExecuteMsg;
} | {
  update_info: UpdateContractMsg;
};
export type PaymentExecuteMsg = {
  update_offering_payment: Payment;
} | {
  update_auction_payment: Payment;
} | {
  remove_offering_payment: {
    contract_addr: Addr;
    sender?: Addr | null;
    token_id: string;
  };
} | {
  remove_auction_payment: {
    contract_addr: Addr;
    sender?: Addr | null;
    token_id: string;
  };
};
export type AssetInfo = {
  token: {
    contract_addr: Addr;
  };
} | {
  native_token: {
    denom: string;
  };
};
export interface Payment {
  asset_info: AssetInfo;
  contract_addr: Addr;
  sender?: Addr | null;
  token_id: string;
}
export interface UpdateContractMsg {
  creator?: Addr | null;
  default_denom?: string | null;
  governance?: Addr | null;
}
export type QueryMsg = {
  msg: PaymentQueryMsg;
} | {
  get_contract_info: {};
};
export type PaymentQueryMsg = {
  get_offering_payment: {
    contract_addr: Addr;
    sender?: Addr | null;
    token_id: string;
  };
} | {
  get_offering_payments: {
    limit?: number | null;
    offset?: Binary | null;
    order?: number | null;
  };
} | {
  get_auction_payment: {
    contract_addr: Addr;
    sender?: Addr | null;
    token_id: string;
  };
} | {
  get_auction_payments: {
    limit?: number | null;
    offset?: Binary | null;
    order?: number | null;
  };
} | {
  get_contract_info: {};
};
export type Binary = string;
export interface ContractInfo {
  creator: Addr;
  default_denom: string;
  governance: Addr;
}
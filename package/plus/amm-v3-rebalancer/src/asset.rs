use cosmos_sdk_proto::{
    cosmos::{authz::v1beta1::MsgExec, bank::v1beta1::MsgSend, base::v1beta1::Coin as ProtoCoin},
    traits::{Message, MessageExt},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Env, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use oraiswap_v3_common::asset::AssetInfo;

use crate::error::ContractError;

#[cw_serde]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

impl Asset {
    pub fn new(info: AssetInfo, amount: Uint128) -> Self {
        Self { info, amount }
    }

    pub fn transfer(
        &self,
        msgs: &mut Vec<CosmosMsg>,
        recipient: String,
    ) -> Result<(), ContractError> {
        if !self.amount.is_zero() {
            match &self.info {
                AssetInfo::Token { contract_addr } => {
                    msgs.push(
                        WasmMsg::Execute {
                            contract_addr: contract_addr.to_string(),
                            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                                recipient,
                                amount: self.amount,
                            })?,
                            funds: vec![],
                        }
                        .into(),
                    );
                }
                AssetInfo::NativeToken { denom } => msgs.push(
                    BankMsg::Send {
                        to_address: recipient,
                        amount: vec![Coin {
                            amount: self.amount,
                            denom: denom.to_string(),
                        }],
                    }
                    .into(),
                ),
            }
        }
        Ok(())
    }

    pub fn transfer_from(
        &self,
        env: &Env,
        msgs: &mut Vec<CosmosMsg>,
        allower: String,
        recipient: String,
    ) -> Result<(), ContractError> {
        if !self.amount.is_zero() {
            match &self.info {
                AssetInfo::Token { contract_addr } => {
                    msgs.push(
                        WasmMsg::Execute {
                            contract_addr: contract_addr.to_string(),
                            msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
                                owner: allower.to_string(),
                                recipient,
                                amount: self.amount,
                            })?,
                            funds: vec![],
                        }
                        .into(),
                    );
                }
                AssetInfo::NativeToken { denom } => {
                    let send = MsgSend {
                        from_address: allower.to_string(),
                        to_address: recipient.to_string(),
                        amount: vec![ProtoCoin {
                            denom: denom.clone(),
                            amount: self.amount.to_string(),
                        }],
                    };
                    let send_any_result = send.to_any();
                    if send_any_result.is_err() {
                        return Ok(());
                    }
                    let stargate_value = Binary::from(
                        MsgExec {
                            grantee: env.contract.address.to_string(),
                            msgs: vec![send_any_result.unwrap()],
                        }
                        .encode_to_vec(),
                    );

                    let stargate = CosmosMsg::Stargate {
                        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
                        value: stargate_value,
                    };
                    msgs.push(stargate)
                }
            }
        }

        Ok(())
    }
}

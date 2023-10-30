use std::env::current_dir;
use std::fs::create_dir_all;

use aioracle_base::Reward;
use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cosmwasm_std::Coin;
use provider_bridge::{
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::Contracts,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema_with_title(&mut schema_for!(InitMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&mut schema_for!(HandleMsg), &out_dir, "ExecuteMsg");
    export_schema(&schema_for!(QueryMsg), &out_dir);

    // types export
    export_schema_with_title(
        &mut schema_for!(Contracts),
        &out_dir,
        "ServiceContractsMsgResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Vec<Reward>),
        &out_dir,
        "ServiceFeeMsgResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Coin),
        &out_dir,
        "GetParticipantFeeResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Coin),
        &out_dir,
        "GetBoundExecutorFeeResponse",
    );
}

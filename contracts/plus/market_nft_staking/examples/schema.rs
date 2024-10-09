use std::{env::current_dir, fs::create_dir_all};

use cosmwasm_schema::{export_schema, schema_for};
use market_nft_staking::{
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::ContractInfo,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    export_schema(&schema_for!(ContractInfo), &out_dir);
}

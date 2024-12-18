use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use market_ai_royalty::{AiRoyaltyExecuteMsg, AiRoyaltyQueryMsg, RoyaltyMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(AiRoyaltyExecuteMsg), &out_dir);
    export_schema(&schema_for!(AiRoyaltyQueryMsg), &out_dir);
    export_schema(&schema_for!(RoyaltyMsg), &out_dir);
}

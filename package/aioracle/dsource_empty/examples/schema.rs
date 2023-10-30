use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cosmwasm_std::HumanAddr;
use provider::msg::{HandleMsg, InitMsg, QueryMsg};
use provider::state::State;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema_with_title(&mut schema_for!(InitMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&mut schema_for!(HandleMsg), &out_dir, "ExecuteMsg");

    // export types
    export_schema_with_title(&mut schema_for!(State), &out_dir, "GetStateResponse");
    export_schema_with_title(&mut schema_for!(HumanAddr), &out_dir, "GetOwnerResponse");
}

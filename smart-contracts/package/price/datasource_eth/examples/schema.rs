use std::env::current_dir;
use std::env::var;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use datasource_eth::msg::{HandleMsg, InitMsg, QueryMsg, SpecialQuery};

fn main() {
    let mut out_dir = current_dir().unwrap();
    if let Ok(artifacts_path) = var("ARTIFACTS_PATH") {
        out_dir.push(artifacts_path);
    }
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(SpecialQuery), &out_dir);
}

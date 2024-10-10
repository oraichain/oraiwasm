use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use market::{AdminList, Registry};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    // export_schema(&schema_for!(Auction), &out_dir);
    export_schema(&schema_for!(AdminList), &out_dir);
    export_schema(&schema_for!(Registry), &out_dir);
}

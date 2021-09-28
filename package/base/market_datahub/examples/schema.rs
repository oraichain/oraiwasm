use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use market_datahub::{Annotation, DataHubHandleMsg, DataHubQueryMsg, Offering};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(Offering), &out_dir);
    export_schema(&schema_for!(DataHubHandleMsg), &out_dir);
    export_schema(&schema_for!(Annotation), &out_dir);
    export_schema(&schema_for!(DataHubQueryMsg), &out_dir);
}

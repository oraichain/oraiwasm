use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use market_first_lv_royalty::{
    FirstLvRoyalty, FirstLvRoyaltyHandleMsg, FirstLvRoyaltyQueryMsg, FirstLvsResponse, InfoMsg,
    QueryFirstLvResult,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(FirstLvRoyalty), &out_dir);
    export_schema(&schema_for!(FirstLvRoyaltyQueryMsg), &out_dir);
    export_schema(&schema_for!(FirstLvsResponse), &out_dir);
    export_schema(&schema_for!(QueryFirstLvResult), &out_dir);
    export_schema(&schema_for!(InfoMsg), &out_dir);
    export_schema(&schema_for!(FirstLvRoyaltyHandleMsg), &out_dir);
}

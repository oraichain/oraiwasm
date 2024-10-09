use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use market_first_level_royalty_storage::msg::{HandleMsg, InitMsg, QueryMsg};
use market_first_level_royalty_storage::state::ContractInfo;
use market_first_lv_royalty::{FirstLvRoyalty, FirstLvRoyaltyHandleMsg, FirstLvRoyaltyQueryMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(FirstLvRoyalty), &out_dir);
    export_schema(&schema_for!(ContractInfo), &out_dir);
    export_schema(&schema_for!(FirstLvRoyaltyHandleMsg), &out_dir);
    export_schema(&schema_for!(FirstLvRoyaltyQueryMsg), &out_dir);
}

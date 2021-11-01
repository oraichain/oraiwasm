use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(
        &schema_for!(market_whitelist::MarketWhiteListHandleMsg),
        &out_dir,
    );
    export_schema(
        &schema_for!(market_whitelist::MarketWhiteListdQueryMsg),
        &out_dir,
    );
    export_schema(
        &schema_for!(market_whitelist::ApprovedForAllResponse),
        &out_dir,
    );
    export_schema(
        &schema_for!(market_whitelist::IsApprovedForAllResponse),
        &out_dir,
    );
}

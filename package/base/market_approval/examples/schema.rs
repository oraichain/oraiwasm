use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(
        &schema_for!(market_approval::MarketApprovalHandleMsg),
        &out_dir,
    );
    export_schema(
        &schema_for!(market_approval::MarketApprovalQueryMsg),
        &out_dir,
    );
    export_schema(
        &schema_for!(market_approval::ApprovedForAllResponse),
        &out_dir,
    );
    export_schema(
        &schema_for!(market_approval::IsApprovedForAllResponse),
        &out_dir,
    );
}

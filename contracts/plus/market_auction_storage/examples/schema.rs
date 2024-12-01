use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use market_auction::{AuctionQueryMsg, AuctionsResponse};
use market_auction_storage::msg::{ExecuteMsg, InstantiateMsg};
use market_auction_storage::state::ContractInfo;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(AuctionQueryMsg), &out_dir);
    export_schema(&schema_for!(AuctionsResponse), &out_dir);
    export_schema(&schema_for!(ContractInfo), &out_dir);
}

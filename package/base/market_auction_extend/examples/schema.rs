use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use market_auction_extend::{Auction, AuctionHandleMsg, AuctionQueryMsg, AuctionsResponse, PagingOptions};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(Auction), &out_dir);
    export_schema(&schema_for!(AuctionHandleMsg), &out_dir);
    export_schema(&schema_for!(AuctionQueryMsg), &out_dir);
    export_schema(&schema_for!(AuctionsResponse), &out_dir);
    export_schema(&schema_for!(PagingOptions), &out_dir);
}

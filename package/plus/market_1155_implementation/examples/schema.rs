use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cosmwasm_std::Uint128;
use market_1155::Offering;
use market_1155_implementation::msg::{HandleMsg, InitMsg, MigrateMsg, QueryMsg};
use market_1155_implementation::state::ContractInfo;
use market_ai_royalty::Royalty;
use market_auction_extend::Auction;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema_with_title(&mut schema_for!(InitMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&mut schema_for!(HandleMsg), &out_dir, "ExecuteMsg");
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);

    export_schema_with_title(&mut schema_for!(ContractInfo), &out_dir, "GetContractInfo");

    export_schema_with_title(&mut schema_for!(Uint128), &out_dir, "GetMarketFees");

    export_schema_with_title(&mut schema_for!(Offering), &out_dir, "Offering");

    export_schema_with_title(&mut schema_for!(Royalty), &out_dir, "AiRoyalty");

    export_schema_with_title(&mut schema_for!(Auction), &out_dir, "Auction");
}

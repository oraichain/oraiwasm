use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use market_royalty::{
    InfoMsg, Offering, OfferingHandleMsg, OfferingQueryMsg, OfferingsResponse, QueryOfferingsResult,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(Offering), &out_dir);
    export_schema(&schema_for!(OfferingQueryMsg), &out_dir);
    export_schema(&schema_for!(OfferingsResponse), &out_dir);
    export_schema(&schema_for!(QueryOfferingsResult), &out_dir);
    export_schema(&schema_for!(InfoMsg), &out_dir);
    export_schema(&schema_for!(OfferingHandleMsg), &out_dir);
}

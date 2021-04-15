use std::env::{current_dir, var};
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw721::{
    AllNftInfoResponse, ApprovedForAllResponse, ContractInfoResponse, NftInfoResponse,
    NumTokensResponse, OwnerOfResponse, TokensResponse,
};
use nft::msg::{HandleMsg, InitMsg, MinterResponse, QueryMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    if let Ok(artifacts_path) = var("ARTIFACTS_PATH") {
        out_dir.push(artifacts_path);
    }
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(AllNftInfoResponse), &out_dir);
    export_schema(&schema_for!(ApprovedForAllResponse), &out_dir);
    export_schema(&schema_for!(ContractInfoResponse), &out_dir);
    export_schema(&schema_for!(MinterResponse), &out_dir);
    export_schema(&schema_for!(NftInfoResponse), &out_dir);
    export_schema(&schema_for!(NumTokensResponse), &out_dir);
    export_schema(&schema_for!(OwnerOfResponse), &out_dir);
    export_schema(&schema_for!(TokensResponse), &out_dir);
}

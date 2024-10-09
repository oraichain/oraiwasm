use std::env::current_dir;
use std::fs::create_dir_all;

use aioracle_base::ServiceFeesResponse;
use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use aioracle_service_fees::msg::*;
use aioracle_service_fees::state::ContractInfo;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema_with_title(&mut schema_for!(InstantiateMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&mut schema_for!(ExecuteMsg), &out_dir, "ExecuteMsg");
    export_schema(&schema_for!(QueryMsg), &out_dir);

    // export types
    export_schema_with_title(
        &mut schema_for!(Vec<ServiceFeesResponse>),
        &out_dir,
        "GetListServiceFeesResponse",
    );
    export_schema_with_title(
        &mut schema_for!(ServiceFeesResponse),
        &out_dir,
        "GetServiceFeesResponse",
    );
    export_schema_with_title(
        &mut schema_for!(ContractInfo),
        &out_dir,
        "GetContractInfoResponse",
    );
}

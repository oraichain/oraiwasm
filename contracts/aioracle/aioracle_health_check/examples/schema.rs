use std::env::{current_dir, var};
use std::fs::create_dir_all;

use aioracle_health_check::state::{ReadPingInfo, State};
use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use aioracle_health_check::msg::{
    HandleMsg, InitMsg, MigrateMsg, QueryMsg, QueryPingInfoResponse, QueryPingInfosResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    if let Ok(artifacts_path) = var("ARTIFACTS_PATH") {
        out_dir.push(artifacts_path);
    }
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
    export_schema_with_title(&mut schema_for!(InitMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&mut schema_for!(HandleMsg), &out_dir, "ExecuteMsg");

    // export types

    export_schema_with_title(
        &mut schema_for!(QueryPingInfoResponse),
        &out_dir,
        "GetPingInfoResponse",
    );
    export_schema_with_title(
        &mut schema_for!(ReadPingInfo),
        &out_dir,
        "GetReadPingInfoResponse",
    );
    export_schema_with_title(&mut schema_for!(State), &out_dir, "GetStateResponse");
    export_schema_with_title(
        &mut schema_for!(Vec<QueryPingInfosResponse>),
        &out_dir,
        "GetPingInfosResponse",
    );
}

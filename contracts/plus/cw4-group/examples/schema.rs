use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

pub use cw4::{AdminResponse, MemberListResponse, MemberResponse, TotalWeightResponse};
pub use cw4_group::msg::{HandleMsg, InitMsg, QueryMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema_with_title(&mut schema_for!(InitMsg), &out_dir, "InitMsg");
    export_schema_with_title(&mut schema_for!(HandleMsg), &out_dir, "HandleMsg");
    export_schema_with_title(&mut schema_for!(QueryMsg), &out_dir, "QueryMsg");
    export_schema(&schema_for!(AdminResponse), &out_dir);
    export_schema(&schema_for!(MemberListResponse), &out_dir);
    export_schema(&schema_for!(MemberResponse), &out_dir);
    export_schema(&schema_for!(TotalWeightResponse), &out_dir);
}

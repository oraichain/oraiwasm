use std::env::current_dir;
use std::fs::create_dir_all;

use aioracle_base::{Executor, Reward};
use aioracle_v2::{
    msg::{
        ExecuteMsg, InstantiateMsg, IsClaimedResponse, LatestStageResponse, MigrateMsg, QueryMsg,
        RequestResponse, StageInfo, TrustingPoolResponse,
    },
    state::{Config, Contracts},
};
use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};
use cosmwasm_std::Coin;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
    export_schema_with_title(&mut schema_for!(InstantiateMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&mut schema_for!(ExecuteMsg), &out_dir, "ExecuteMsg");

    // export query types
    export_schema(&schema_for!(IsClaimedResponse), &out_dir);
    export_schema(&schema_for!(LatestStageResponse), &out_dir);
    export_schema_with_title(&mut schema_for!(Config), &out_dir, "ConfigResponse");
    export_schema_with_title(&mut schema_for!(bool), &out_dir, "VerifyDataResponse");
    export_schema_with_title(
        &mut schema_for!(Vec<Executor>),
        &out_dir,
        "GetExecutorsResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Vec<Executor>),
        &out_dir,
        "GetExecutorsByIndexResponse",
    );
    export_schema_with_title(&mut schema_for!(Executor), &out_dir, "GetExecutorResponse");
    export_schema_with_title(&mut schema_for!(u64), &out_dir, "GetExecutorSizeResponse");
    export_schema_with_title(
        &mut schema_for!(RequestResponse),
        &out_dir,
        "GetRequestResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Vec<RequestResponse>),
        &out_dir,
        "GetRequestsResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Vec<RequestResponse>),
        &out_dir,
        "GetRequestsByServiceResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Vec<RequestResponse>),
        &out_dir,
        "GetRequestsByMerkleRootResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Contracts),
        &out_dir,
        "GetServiceContractsResponse",
    );
    export_schema_with_title(&mut schema_for!(StageInfo), &out_dir, "StageInfoResponse");
    export_schema_with_title(
        &mut schema_for!(Vec<Reward>),
        &out_dir,
        "GetServiceFeesResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Coin),
        &out_dir,
        "GetBoundExecutorFeeResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Coin),
        &out_dir,
        "GetParticipantFeeResponse",
    );
    export_schema_with_title(
        &mut schema_for!(TrustingPoolResponse),
        &out_dir,
        "GetTrustingPoolResponse",
    );
    export_schema_with_title(
        &mut schema_for!(Vec<TrustingPoolResponse>),
        &out_dir,
        "GetTrustingPoolsResponse",
    );
}

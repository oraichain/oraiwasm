use crate::msg::{AIRequest, ContractInfoResponse};
use cw_storage_plus::{Item, Map};

pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("contract_info");
pub const AIREQUESTS: Map<&str, AIRequest> = Map::new("airequest");

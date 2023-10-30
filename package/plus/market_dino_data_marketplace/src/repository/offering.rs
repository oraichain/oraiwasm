use cosmwasm_std::{DepsMut, HumanAddr, StdResult};
use cw_storage_plus::{IndexedMap, Map};

use crate::{
    model::offering::{OwnershipOffering, UsageOffering, UsageOfferingSold},
    storage::offering::UsageOfferingSoldIndexes,
};

pub struct OfferingRepository {
    storage_ownership_offerings: Map<'static, &'static str, OwnershipOffering>,
    storage_usage_offerings: Map<'static, &'static str, UsageOffering>,
    storage_usage_offering_solds:
        IndexedMap<'static, &'static [u8], UsageOfferingSold, UsageOfferingSoldIndexes<'static>>,
}

impl OfferingRepository {
    pub fn find_ownership_offering_by_token_id(
        &self,
        deps: DepsMut,
        token_id: &str,
    ) -> StdResult<OwnershipOffering> {
        self.storage_ownership_offerings
            .load(deps.storage, token_id)
    }

    pub fn find_usage_offering_by_token_id(
        &self,
        deps: DepsMut,
        token_id: &str,
    ) -> StdResult<UsageOffering> {
        self.storage_usage_offerings.load(deps.storage, token_id)
    }

    pub fn find_usage_offering_solds(
        &self,
        deps: DepsMut,
        offering_id: String,
        buyer: HumanAddr,
        version: String,
    ) -> StdResult<UsageOfferingSold> {
        self.storage_usage_offering_solds.load(
            deps.storage,
            &UsageOfferingSold::get_id(offering_id, buyer, version)
                .as_bytes()
                .to_vec(),
        )
    }
}

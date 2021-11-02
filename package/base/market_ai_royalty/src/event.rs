use cosmwasm_std::{attr, HandleResponse, Uint128};

pub trait Event {
    /// Append attributes to response
    fn add_attributes(&self, response: &mut HandleResponse);
}

pub struct RoyaltyEvent {
    pub creator: String,
    pub royalty: u64,
    pub amount: Uint128,
    pub denom: String,
}

pub struct RoyaltiesEvent<'a> {
    pub royalties_event: &'a [RoyaltyEvent],
}

impl<'a> Event for RoyaltiesEvent<'a> {
    fn add_attributes(&self, rsp: &mut HandleResponse) {
        rsp.attributes.push(attr("action", "pay_royalty"));
        for royalty in self.royalties_event {
            rsp.attributes.push(attr(
                format!("royalty_{}_{}", royalty.creator, royalty.royalty),
                format!("{}{}", royalty.amount, royalty.denom),
            ));
        }
        rsp.attributes.push(attr("action", "finish_pay_royalty"));
    }
}

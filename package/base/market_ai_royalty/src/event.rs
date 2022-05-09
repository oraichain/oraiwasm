use cosmwasm_std::{attr, HandleResponse, Uint128};

pub trait Event {
    /// Append attributes to response
    fn add_attributes(&self, response: &mut HandleResponse);
}

pub struct RoyaltyEvent<'a> {
    pub creator: &'a str,
    pub royalty: u64,
    pub amount: Uint128,
    pub denom: &'a str,
}

pub struct RoyaltiesEvent<'a> {
    pub nft_addr: &'a str,
    pub token_id: &'a str,
    pub royalties_event: &'a [RoyaltyEvent<'a>],
}

impl<'a> Event for RoyaltiesEvent<'a> {
    fn add_attributes(&self, rsp: &mut HandleResponse) {
        rsp.attributes.push(attr("nft_addr", self.nft_addr));
        rsp.attributes.push(attr("token_id", self.token_id));
        rsp.attributes.push(attr("action", "pay_royalty"));
        for royalty in self.royalties_event {
            rsp.attributes.push(attr(
                format!("royalty_{}_{}", royalty.creator, royalty.royalty),
                format!("{}_{}", royalty.amount, royalty.denom),
            ));
        }
        rsp.attributes.push(attr("action", "finish_pay_royalty"));
    }
}

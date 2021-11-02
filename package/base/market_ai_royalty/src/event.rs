use cosmwasm_std::{attr, HandleResponse};

use crate::Royalty;

pub trait Event {
    /// Append attributes to response
    fn add_attributes(&self, response: &mut HandleResponse);
}

pub struct RoyaltiesEvent<'a> {
    pub royalties: &'a [Royalty],
}

impl<'a> Event for RoyaltiesEvent<'a> {
    fn add_attributes(&self, rsp: &mut HandleResponse) {
        rsp.attributes.push(attr("action", "pay_royalty"));
        for royalty in self.royalties {
            rsp.attributes.push(attr(
                format!("royalty_{}", royalty.creator),
                royalty.royalty,
            ));
        }
        rsp.attributes.push(attr("action", "finish_royalty"));
    }
}

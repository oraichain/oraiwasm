use cosmwasm_std::{attr, HandleResponse};

pub trait Event {
    /// Append attributes to response
    fn add_attributes(&self, response: &mut HandleResponse);
}

/// Tracks approve_all status changes
pub struct ApproveAllEvent<'a> {
    pub sender: &'a str,
    pub nft_addr: &'a str,
    pub approved: bool,
}

impl<'a> Event for ApproveAllEvent<'a> {
    fn add_attributes(&self, rsp: &mut HandleResponse) {
        rsp.attributes.push(attr("action", "approve_all"));
        rsp.attributes.push(attr("sender", self.sender.to_string()));
        rsp.attributes
            .push(attr("nft_addr", self.nft_addr.to_string()));
        rsp.attributes
            .push(attr("approved", (self.approved as u32).to_string()));
    }
}

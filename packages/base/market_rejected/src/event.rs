use cosmwasm_std::{attr, HandleResponse};

pub trait Event {
    /// Append attributes to response
    fn add_attributes(&self, response: &mut HandleResponse);
}

/// Tracks approve_all status changes
pub struct RejectAllEvent<'a> {
    pub sender: &'a str,
    pub contract_addr: &'a str,
    pub token_id: &'a str,
    pub rejected: bool,
}

impl<'a> Event for RejectAllEvent<'a> {
    fn add_attributes(&self, rsp: &mut HandleResponse) {
        rsp.attributes.push(attr("action", "approve_all"));
        rsp.attributes.push(attr("sender", self.sender.to_string()));
        rsp.attributes
            .push(attr("contract_addr", self.contract_addr.to_string()));
        rsp.attributes
            .push(attr("token_id", self.token_id.to_string()));
        rsp.attributes
            .push(attr("rejected", (self.rejected as u32).to_string()));
    }
}

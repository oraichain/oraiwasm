use cosmwasm_std::{attr, HandleResponse, Uint128};

pub trait Event {
    /// Append attributes to response
    fn add_attributes(&self, response: &mut HandleResponse);
}

/// Tracks token transfer/mint/burn actions
pub struct TransferEvent<'a> {
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub token_id: &'a str,
    pub amount: Uint128,
}

impl<'a> Event for TransferEvent<'a> {
    fn add_attributes(&self, rsp: &mut HandleResponse) {
        rsp.attributes.push(attr("action", "transfer"));
        rsp.attributes.push(attr("token_id", self.token_id));
        rsp.attributes.push(attr("amount", self.amount));
        if let Some(from) = self.from {
            rsp.attributes.push(attr("from", from.to_string()));
        }
        if let Some(to) = self.to {
            rsp.attributes.push(attr("to", to.to_string()));
        }
    }
}

/// Tracks token metadata changes
pub struct MetadataEvent<'a> {
    pub url: &'a str,
    pub token_id: &'a str,
}

impl<'a> Event for MetadataEvent<'a> {
    fn add_attributes(&self, rsp: &mut HandleResponse) {
        rsp.attributes.push(attr("action", "set_metadata"));
        rsp.attributes.push(attr("url", self.url));
        rsp.attributes.push(attr("token_id", self.token_id));
    }
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

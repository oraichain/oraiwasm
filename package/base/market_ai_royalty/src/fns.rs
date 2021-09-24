use cosmwasm_std::StdError;

pub fn sanitize_royalty(royalty: u64, limit: u64, name: &str) -> Result<u64, StdError> {
    if royalty > limit {
        return Err(StdError::GenericErr {
            msg: format!("Invalid argument: {}", name.to_string()),
        });
    }
    Ok(royalty)
}

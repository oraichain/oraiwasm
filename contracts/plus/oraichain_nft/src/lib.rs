pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

#[macro_export]
macro_rules! check_size {
    ($arg:ident, $len:expr) => {{
        if $arg.len() > $len {
            return Err(ContractError::InvalidArgument {
                reason: format!("`{}` exceeds {} chars", stringify!($arg), $len),
            });
        }
    }};
}

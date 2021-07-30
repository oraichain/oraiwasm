use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Fraction {
    /// The *numerator* of this `Fraction`.
    pub nom: Uint128,
    /// The *denominator* of this `Fraction`.
    pub denom: Uint128,
}

impl Fraction {
    /// Checks the given `Fraction` is valid, *i.e.*,
    /// - Has a non-zero denominator, and
    /// - The `nom` is less than `denom`.
    pub fn check(&self) -> bool {
        !self.denom.is_zero() && self.nom < self.denom
    }

    pub fn is_zero(&self) -> bool {
        self.nom.is_zero()
    }

    /// Multiplies this `Fraction` by the given `value`.
    pub fn multiply(&self, value: Uint128) -> Uint128 {
        value.multiply_ratio(self.nom, self.denom)
    }
}

impl Display for Fraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.nom, self.denom)
    }
}

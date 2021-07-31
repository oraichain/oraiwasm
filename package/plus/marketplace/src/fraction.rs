use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Write},
    ops::{Add, Sub},
};

const DECIMAL_FRACTIONAL: u128 = 1_000_000;

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
    /// - The `nom` is less than or equal `denom`.
    pub fn check(&self) -> bool {
        !self.denom.is_zero() && self.nom.le(&self.denom)
    }

    pub fn is_zero(&self) -> bool {
        self.nom.is_zero()
    }

    /// Multiplies this `Fraction` by the given `value`.
    pub fn multiply(&self, value: Uint128) -> Uint128 {
        value.multiply_ratio(self.nom, self.denom)
    }

    pub fn mul(&self, other: &Self) -> Self {
        Fraction {
            nom: other.multiply(self.nom),
            denom: self.denom,
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        let one = Uint128(1);
        Fraction {
            nom: self
                .nom
                .multiply_ratio(other.denom, one)
                .sub(self.denom.multiply_ratio(other.nom, one))
                // must >= 0 for Uint128
                .unwrap_or(Uint128::zero()),
            denom: self.denom.multiply_ratio(other.denom, one),
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        let one = Uint128(1);
        Fraction {
            nom: self
                .nom
                .multiply_ratio(other.denom, one)
                .add(self.denom.multiply_ratio(other.nom, one)),
            denom: self.denom.multiply_ratio(other.denom, one),
        }
    }

    pub fn one() -> Self {
        Fraction {
            nom: Uint128(1),
            denom: Uint128(1),
        }
    }
}

impl Display for Fraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (val, fraction) = (
            self.nom.u128() * DECIMAL_FRACTIONAL * 100,
            DECIMAL_FRACTIONAL * self.denom.u128(),
        );
        let whole = val / fraction;
        let fractional = val % fraction;

        if fractional == 0 {
            write!(f, "{}%", whole)
        } else {
            let fractional_string = format!("{:06}", fractional);
            f.write_str(&whole.to_string())?;
            f.write_char('.')?;
            f.write_str(fractional_string.trim_end_matches('0'))?;
            write!(f, "%")
        }
    }
}

pub type Payout = (HumanAddr, Fraction);
// show as percentage %
pub type PrettyPayout = (String, String);

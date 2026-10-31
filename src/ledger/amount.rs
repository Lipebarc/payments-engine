use std::fmt;
use std::ops::Neg;
use std::str::FromStr;

use rust_decimal::Decimal;

use crate::error::ParseError;

/// Number of decimal places accepted on input and printed on output.
pub const PRECISION: u32 = 4;

/// Exact signed monetary value with at most [`PRECISION`] decimal places.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(Decimal);

impl Amount {
    pub const ZERO: Self = Self(Decimal::ZERO);

    pub fn is_positive(self) -> bool {
        self.0 > Decimal::ZERO
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }
}

impl Neg for Amount {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl FromStr for Amount {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseError::InvalidAmount(s.to_owned());
        let value = Decimal::from_str(s).map_err(|_| invalid())?.normalize();
        if value.scale() > PRECISION {
            return Err(invalid());
        }
        Ok(Self(value))
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.*}", PRECISION as usize, self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amount(s: &str) -> Amount {
        s.parse().unwrap()
    }

    #[test]
    fn parses_up_to_four_decimal_places() {
        assert_eq!(amount("1.2345").to_string(), "1.2345");
        assert_eq!(amount("2").to_string(), "2.0000");
        assert_eq!(amount("1.50000").to_string(), "1.5000");
    }

    #[test]
    fn rejects_excess_precision_and_garbage() {
        assert!("1.23456".parse::<Amount>().is_err());
        assert!("abc".parse::<Amount>().is_err());
        assert!("".parse::<Amount>().is_err());
    }

    #[test]
    fn arithmetic_is_exact() {
        let sum = amount("0.1").checked_add(amount("0.2")).unwrap();
        assert_eq!(sum, amount("0.3"));
        assert_eq!(
            amount("1").checked_add(-amount("1.5")).unwrap().to_string(),
            "-0.5000"
        );
    }

    #[test]
    fn detects_overflow() {
        let max = Amount(Decimal::MAX);
        assert_eq!(max.checked_add(amount("1")), None);
    }

    #[test]
    fn only_strictly_positive_values_are_positive() {
        assert!(amount("0.0001").is_positive());
        assert!(!Amount::ZERO.is_positive());
        assert!(!amount("-1").is_positive());
    }
}

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Store the amount of assets in account.
/// It is using internally an i64 in order to avoid floating point rounding error.
/// The i64 (8 bytes) has a smaller memory footprint than BigNumber/Decimal crates.
/// The Amount precision is four places past the decimal
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount {
    store: i64,
}

#[derive(Error, Debug, Clone)]
pub enum AmountError {
    #[error("Amount parsing error: {0}")]
    Parse(String),

    #[error("Overflow error while creating Amount")]
    Overflow,

    #[error("Underflow error while creating Amount")]
    Underflow,
}

impl Amount {
    pub fn new() -> Self {
        Amount { store: 0 }
    }

    pub fn add(&self, other: &Amount) -> Result<Amount, AmountError> {
        match self.store.checked_add(other.store) {
            Some(total) => Ok(Amount { store: total }),
            None => Err(AmountError::Overflow)?,
        }
    }

    pub fn sub(&self, other: &Amount) -> Result<Amount, AmountError> {
        match self.store.checked_sub(other.store) {
            Some(total) => Ok(Amount { store: total }),
            None => Err(AmountError::Underflow)?,
        }
    }
}

impl FromStr for Amount {
    type Err = AmountError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            Err(AmountError::Parse(s.into()))?
        }

        let mut parts = s.split('.');
        let left_part = parts.next().unwrap(); // Ok to unwrap as the first part always exists
        let decimal_part = parts.next();

        // Checking for extra '.'
        if parts.next().is_some() {
            Err(AmountError::Parse(s.into()))?
        }

        // Checking if integer part is empty (ex: ".05")
        let left_str = if left_part.is_empty() { "0" } else { left_part };

        let total: i64 = match decimal_part {
            None => {
                // No decimal part - try to convert and multiply 10000
                let parsed = left_str.parse::<i64>();
                match parsed {
                    Ok(v) => match v.checked_mul(10_000) {
                        Some(val) => val,
                        None => Err(AmountError::Overflow)?, // Overflow when multiplying
                    },
                    Err(_) => Err(AmountError::Parse(s.into()))?,
                }
            }
            Some(dec_str) => {
                let mut dec_str = dec_str.to_owned();
                if dec_str.is_empty() {
                    dec_str = String::from("0000");
                }
                if !dec_str.chars().all(|c| c.is_ascii_digit()) {
                    Err(AmountError::Parse(s.into()))?
                }

                // Ensure 4 digits for decimal part
                if dec_str.len() > 4 {
                    dec_str.truncate(4);
                } else if dec_str.len() < 4 {
                    while dec_str.len() < 4 {
                        dec_str.push('0');
                    }
                }

                let combined_str = format!("{}{}", left_str, dec_str);
                let total = combined_str.parse::<i64>();

                match total {
                    Ok(v) => v,
                    Err(_) => Err(AmountError::Parse(s.into()))?,
                }
            }
        };

        Ok(Self { store: total })
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.store;
        let negative = value < 0;
        let abs_val = value.abs();

        let left_part = abs_val / 10_000;
        let decimal_part = abs_val % 10_000;

        if negative {
            write!(f, "-{}.{:04}", left_part, decimal_part)
        } else {
            write!(f, "{}.{:04}", left_part, decimal_part)
        }
    }
}

mod tests {
    use std::str::FromStr;

    use crate::engine::amount::{self, Amount, AmountError};

    #[test]
    fn test_that_valid_string_can_be_parsed() {
        let amount = Amount::from_str("0");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 0);

        let amount = Amount::from_str("0.");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 0);

        let amount = Amount::from_str(".0");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 0);

        let amount = Amount::from_str("0.005");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 50);

        let amount = Amount::from_str("5");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 50000);

        let amount = Amount::from_str("5.1");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 51000);

        let amount = Amount::from_str("5.123");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 51230);

        let amount = Amount::from_str("5.123456");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 51234);

        let amount = Amount::from_str(".05");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 500);

        let amount = Amount::from_str("-.05");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, -500);

        let amount = Amount::from_str("05.05");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, 50500);

        let amount = Amount::from_str("-12345.1234567");
        assert!(amount.is_ok());
        assert_eq!(amount.unwrap().store, -123451234);
    }

    #[test]
    pub fn test_that_invalid_string_parsing_returns_error() {
        let amount = Amount::from_str("test");
        assert!(amount.is_err());
        assert!(matches!(amount.err().unwrap(), AmountError::Parse(_)));

        let amount = Amount::from_str("123.12test");
        assert!(amount.is_err());
        assert!(matches!(amount.err().unwrap(), AmountError::Parse(_)));

        let amount = Amount::from_str("12test.123");
        assert!(amount.is_err());
        assert!(matches!(amount.err().unwrap(), AmountError::Parse(_)));

        let amount = Amount::from_str("1 .1 2");
        assert!(amount.is_err());
        assert!(matches!(amount.err().unwrap(), AmountError::Parse(_)));

        let amount = Amount::from_str("");
        assert!(amount.is_err());
        assert!(matches!(amount.err().unwrap(), AmountError::Parse(_)));

        // Overflow testing
        let amount = Amount::from_str("9223372036854775808");
        assert!(amount.is_err());
        assert!(matches!(amount.err().unwrap(), AmountError::Parse(_)));

        // Max i64, will be * 10_000
        let amount = Amount::from_str("9223372036854775807");
        assert!(amount.is_err());
        assert!(matches!(amount.err().unwrap(), AmountError::Overflow));
    }

    #[test]
    pub fn test_that_amount_can_be_added() {
        let amount = Amount::from_str("200.12");
        assert!(amount.is_ok());

        let amount_2 = Amount::from_str("100.0023");
        assert!(amount_2.is_ok());

        let sum = amount.unwrap().add(&amount_2.unwrap());
        assert!(sum.is_ok());
        assert_eq!(sum.unwrap().to_string(), "300.1223");

        let amount = Amount::from_str("-200.12");
        assert!(amount.is_ok());

        let amount_2 = Amount::from_str("100.0023");
        assert!(amount_2.is_ok());

        let sum = amount.unwrap().add(&amount_2.unwrap());
        assert!(sum.is_ok());
        assert_eq!(sum.unwrap().to_string(), "-100.1177");
    }

    #[test]
    pub fn test_that_amount_can_be_substracted() {
        let amount = Amount::from_str("200.12");
        assert!(amount.is_ok());

        let amount_2 = Amount::from_str("100.0023");
        assert!(amount_2.is_ok());

        let diff = amount.unwrap().sub(&amount_2.unwrap());
        assert!(diff.is_ok());
        assert_eq!(diff.unwrap().to_string(), "100.1177");

        let amount = Amount::from_str("-200.12");
        assert!(amount.is_ok());

        let amount_2 = Amount::from_str("100.0023");
        assert!(amount_2.is_ok());

        let diff = amount.unwrap().sub(&amount_2.unwrap());
        assert!(diff.is_ok());
        assert_eq!(diff.unwrap().to_string(), "-300.1223");
    }

    #[test]
    pub fn test_that_overflow_return_error() {
        let amount = Amount::from_str("922337203685477.5807");
        assert!(amount.is_ok());

        let amount_2 = Amount::from_str("123");
        assert!(amount_2.is_ok());

        let sum = amount.unwrap().add(&amount_2.unwrap());
        assert!(sum.is_err());
        assert!(matches!(sum.err().unwrap(), AmountError::Overflow));
    }

    #[test]
    pub fn test_that_underflow_return_error() {
        let amount = Amount::from_str("-922337203685477.5807");
        assert!(amount.is_ok());

        let amount_2 = Amount::from_str("123");
        assert!(amount_2.is_ok());

        let sum = amount.unwrap().sub(&amount_2.unwrap());
        assert!(sum.is_err());
        assert!(matches!(sum.err().unwrap(), AmountError::Underflow));
    }
}

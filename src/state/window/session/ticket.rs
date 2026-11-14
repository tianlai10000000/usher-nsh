//! Lossless window operation identities; decimal encoding is confined to serde and the bridge.
use serde::{Deserialize, Serialize};

/// Opaque window ticket; serde accepts only canonical unsigned decimal strings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Ticket(u64);

impl Ticket {
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }
}
impl TryFrom<String> for Ticket {
    type Error = super::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let number: u64 = value.parse().map_err(|_| super::Error::InvalidTicket)?;
        if number.to_string() != value {
            return Err(super::Error::InvalidTicket);
        }
        Ok(Self(number))
    }
}
impl From<Ticket> for String {
    fn from(value: Ticket) -> Self {
        value.0.to_string()
    }
}
impl std::fmt::Display for Ticket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tickets_round_trip_without_js_precision_loss_and_reject_noncanonical_inputs() {
        let ticket = Ticket::new(u64::MAX);
        let wire = serde_json::to_string(&ticket).unwrap();
        assert_eq!(wire, serde_json::to_string(&u64::MAX.to_string()).unwrap());
        assert_eq!(serde_json::from_str::<Ticket>(&wire).unwrap(), ticket);
        for value in ["", "01", "+1", "-1", "18446744073709551616"] {
            assert!(Ticket::try_from(value.to_owned()).is_err());
        }
    }
}

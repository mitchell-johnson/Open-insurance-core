//! Address types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressType {
    Residential,
    Mailing,
    Business,
    Billing,
}

/// A postal address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub id: Uuid,
    pub address_type: AddressType,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub state: Option<String>,
    pub postal_code: String,
    pub country: String,
    pub is_primary: bool,
}

impl Address {
    /// Creates a new address
    pub fn new(
        address_type: AddressType,
        line1: impl Into<String>,
        city: impl Into<String>,
        postal_code: impl Into<String>,
        country: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            address_type,
            line1: line1.into(),
            line2: None,
            city: city.into(),
            state: None,
            postal_code: postal_code.into(),
            country: country.into(),
            is_primary: false,
        }
    }

    /// Formats address for display
    pub fn format(&self) -> String {
        let mut lines = vec![self.line1.clone()];
        if let Some(l2) = &self.line2 {
            lines.push(l2.clone());
        }
        let city_line = match &self.state {
            Some(state) => format!("{}, {} {}", self.city, state, self.postal_code),
            None => format!("{} {}", self.city, self.postal_code),
        };
        lines.push(city_line);
        lines.push(self.country.clone());
        lines.join("\n")
    }
}

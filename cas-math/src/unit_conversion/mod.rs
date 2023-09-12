//! A module for generic conversion between units.
//!
//! The [`Measurement`] type packs a numeric value and the unit it represents. It can be converted
//! to other units within the same quantity kind. Here's an example of converting a length from
//! miles to decimeters:
//!
//! ```
//! use cas_math::unit_conversion::{Length, Measurement, Quantity, Unit};
//!
//! let m = Measurement::new(2.0, Unit::new(Quantity::Length(Length::Mile)));
//! let m2 = m.convert(Unit::new(Quantity::Length(Length::Decimeter))).unwrap();
//! assert_eq!(m2.value(), &32186.88);
//! ```
//!
//! Note that the arguments to [`Measurement::new`] and [`Measurement::convert`] accept any type
//! that implements [`Into<Unit>`], which is implemented for all specific units and quantities.
//! This allows you to write the above example more concisely:
//!
//! ```
//! use cas_math::unit_conversion::{Length, Measurement};
//!
//! let m = Measurement::new(2.0, Length::Mile);
//! let m2 = m.convert(Length::Decimeter).unwrap();
//! assert_eq!(m2.value(), &32186.88);
//! ```

pub mod convert;
pub mod unit;

use std::ops::Mul;
pub use unit::{ConversionError, Length, Quantity, Unit};

/// A value and the unit it represents.
///
/// This value can be converted to other units within the same quantity kind.
pub struct Measurement<T> {
    value: T,
    unit: Unit,
}

impl<T> Measurement<T> {
    /// Create a new measurement.
    pub fn new(value: T, unit: impl Into<Unit>) -> Self {
        Self { value, unit: unit.into() }
    }

    /// Get the value of this measurement.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Get the unit of this measurement.
    pub fn unit(&self) -> &Unit {
        &self.unit
    }

    /// Convert this measurement to another unit.
    ///
    /// In general, target units must be the same kind as the source unit, and with the same
    /// power. However, some conversions are allowed between different kinds of units, such as
    /// between cubed length units and volume units.
    pub fn convert(&self, target: impl Into<Unit>) -> Result<Self, ConversionError>
        where T: Copy + Mul<f64, Output = T>,
    {
        let target = target.into();
        Ok(Self {
            value: self.value * self.unit.conversion_factor(target)?,
            unit: target,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unit::Length;

    #[test]
    fn convert_length() {
        let m = Measurement::new(2.0, Length::Mile);
        let m2 = m.convert(Length::Decimeter).unwrap();
        assert_eq!(m2.value(), &32186.88);
    }
}

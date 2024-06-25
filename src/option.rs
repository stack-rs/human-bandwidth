//! Convenience module to allow serialization for `Option`
//!
//! # Example
//!
//! ```
//! use serde::{Serialize, Deserialize};
//! use bandwidth::Bandwidth;
//!
//! #[derive(Serialize, Deserialize)]
//! struct Foo {
//!     #[serde(default)]
//!     #[serde(with = "human_bandwidth::option")]
//!     timeout: Option<Bandwidth>,
//! }
//! ```

use super::serde::Serde;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serializes an `Option<Bandwidth>`
///
/// This function can be used with `serde_derive`'s `with` and
/// `deserialize_with` annotations.
pub fn serialize<T, S>(d: &Option<T>, s: S) -> Result<S::Ok, S::Error>
where
    for<'a> Serde<&'a T>: Serialize,
    S: Serializer,
{
    let nested: Option<Serde<&T>> = d.as_ref().map(Into::into);
    nested.serialize(s)
}

/// Deserialize an `Option<Bandwidth>`
///
/// This function can be used with `serde_derive`'s `with` and
/// `deserialize_with` annotations.
pub fn deserialize<'a, T, D>(d: D) -> Result<Option<T>, D::Error>
where
    Serde<T>: Deserialize<'a>,
    D: Deserializer<'a>,
{
    let got: Option<Serde<T>> = Deserialize::deserialize(d)?;
    Ok(got.map(Serde::into_inner))
}

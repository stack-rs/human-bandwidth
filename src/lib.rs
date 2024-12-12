//! Human-friendly bandwidth parser and formatter
//!
//! Features:
//!
//! * Parses bandwidth in free form like `2Gbps 340Mbps`
//! * Formats bandwidth in similar form `150kbps 24bps`
//!
//! Enable `serde` feature for serde integration.

use std::error::Error as StdError;
use std::fmt;
use std::str::Chars;

#[cfg(feature = "serde")]
pub mod option;
#[cfg(feature = "serde")]
pub mod serde;

/// Reexport module
pub mod re {
    pub use bandwidth;
}

use bandwidth::Bandwidth;

/// Error parsing human-friendly bandwidth
#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    /// Invalid character during parsing
    ///
    /// More specifically anything that is not alphanumeric is prohibited
    ///
    /// The field is an byte offset of the character in the string.
    InvalidCharacter(usize),
    /// Non-numeric value where number is expected
    ///
    /// This usually means that either bandwidth unit is broken into words,
    /// e.g. `M bps` instead of `Mbps`, or just number is omitted,
    /// for example `2 Mbps kbps` instead of `2 Mbps 1 kbps`
    ///
    /// The field is an byte offset of the erroneous character
    /// in the string.
    NumberExpected(usize),
    /// Unit in the number is not one of allowed units
    ///
    /// See documentation of `parse_bandwidth` for the list of supported
    /// bandwidth units.
    ///
    /// The two fields are start and end (exclusive) of the slice from
    /// the original string, containing erroneous value
    UnknownUnit {
        /// Start of the invalid unit inside the original string
        start: usize,
        /// End of the invalid unit inside the original string
        end: usize,
        /// The unit verbatim
        unit: String,
        /// A number associated with the unit
        value: u64,
    },
    /// The numeric value is too large
    ///
    /// Usually this means value is too large to be useful.
    NumberOverflow,
    /// The value was an empty string (or consists only whitespace)
    Empty,
}

impl StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidCharacter(offset) => write!(f, "invalid character at {}", offset),
            Error::NumberExpected(offset) => write!(f, "expected number at {}", offset),
            Error::UnknownUnit { unit, value, .. } if unit.is_empty() => {
                write!(
                    f,
                    "bandwidth unit needed, for example {0}Mbps or {0}bps",
                    value,
                )
            }
            Error::UnknownUnit { unit, .. } => {
                write!(
                    f,
                    "unknown bandwidth unit {:?}, \
                    supported units: bps, kbps, Mbps, Gbps, Tbps",
                    unit
                )
            }
            Error::NumberOverflow => write!(f, "number is too large"),
            Error::Empty => write!(f, "value was empty"),
        }
    }
}

/// A wrapper type that allows you to Display a Bandwidth
#[derive(Debug, Clone)]
pub struct FormattedBandwidth(Bandwidth);

trait OverflowOp: Sized {
    fn mul(self, other: Self) -> Result<Self, Error>;
    fn add(self, other: Self) -> Result<Self, Error>;
}

impl OverflowOp for u64 {
    fn mul(self, other: Self) -> Result<Self, Error> {
        self.checked_mul(other).ok_or(Error::NumberOverflow)
    }
    fn add(self, other: Self) -> Result<Self, Error> {
        self.checked_add(other).ok_or(Error::NumberOverflow)
    }
}

struct Parser<'a> {
    iter: Chars<'a>,
    src: &'a str,
    current: (u64, u64),
}

impl Parser<'_> {
    fn off(&self) -> usize {
        self.src.len() - self.iter.as_str().len()
    }

    fn parse_first_char(&mut self) -> Result<Option<u64>, Error> {
        let off = self.off();
        for c in self.iter.by_ref() {
            match c {
                '0'..='9' => {
                    return Ok(Some(c as u64 - '0' as u64));
                }
                c if c.is_whitespace() => continue,
                _ => {
                    return Err(Error::NumberExpected(off));
                }
            }
        }
        Ok(None)
    }
    fn parse_unit(&mut self, n: u64, start: usize, end: usize) -> Result<(), Error> {
        let (mut gbps, bps) = match &self.src[start..end] {
            "bps" | "bit/s" | "b/s" => (0u64, n),
            "kbps" | "Kbps" | "kbit/s" | "Kbit/s" | "kb/s" | "Kb/s" => (0u64, n.mul(1000)?),
            "Mbps" | "mbps" | "Mbit/s" | "mbit/s" | "Mb/s" | "mb/s" => (0u64, n.mul(1_000_000)?),
            "Gbps" | "gbps" | "Gbit/s" | "gbit/s" | "Gb/s" | "gb/s" => (n, 0),
            "Tbps" | "tbps" | "Tbit/s" | "tbit/s" | "Tb/s" | "tb/s" => (n.mul(1000)?, 0),
            _ => {
                return Err(Error::UnknownUnit {
                    start,
                    end,
                    unit: self.src[start..end].to_string(),
                    value: n,
                });
            }
        };
        let mut bps = self.current.1.add(bps)?;
        if bps > 1_000_000_000 {
            gbps = gbps.add(bps / 1_000_000_000)?;
            bps %= 1_000_000_000;
        }
        gbps = self.current.0.add(gbps)?;
        self.current = (gbps, bps);
        Ok(())
    }

    fn parse(mut self) -> Result<Bandwidth, Error> {
        let mut n = self.parse_first_char()?.ok_or(Error::Empty)?;
        'outer: loop {
            let mut off = self.off();
            while let Some(c) = self.iter.next() {
                match c {
                    '0'..='9' => {
                        n = n
                            .checked_mul(10)
                            .and_then(|x| x.checked_add(c as u64 - '0' as u64))
                            .ok_or(Error::NumberOverflow)?;
                    }
                    c if c.is_whitespace() => {}
                    'a'..='z' | 'A'..='Z' | '/' => {
                        break;
                    }
                    _ => {
                        return Err(Error::InvalidCharacter(off));
                    }
                }
                off = self.off();
            }
            let start = off;
            let mut off = self.off();
            while let Some(c) = self.iter.next() {
                match c {
                    '0'..='9' => {
                        self.parse_unit(n, start, off)?;
                        n = c as u64 - '0' as u64;
                        continue 'outer;
                    }
                    c if c.is_whitespace() => break,
                    'a'..='z' | 'A'..='Z' | '/' => {}
                    _ => {
                        return Err(Error::InvalidCharacter(off));
                    }
                }
                off = self.off();
            }
            self.parse_unit(n, start, off)?;
            n = match self.parse_first_char()? {
                Some(n) => n,
                None => return Ok(Bandwidth::new(self.current.0, self.current.1 as u32)),
            };
        }
    }
}

/// Parse bandwidth object `1Gbps 12Mbps 5bps`
///
/// The bandwidth object is a concatenation of rate spans. Where each rate
/// span is an integer number and a suffix. Supported suffixes:
///
/// * `bps`, `bit/s`, `b/s` -- bit per second
/// * `kbps`, `kbit/s`, `kb/s` -- kilobit per second
/// * `Mbps`, `Mbit/s`, `Mb/s` -- megabit per second
/// * `Gbps`, `Gbit/s`, `Gb/s` -- gigabit per second
/// * `Tbps`, `Tbit/s`, `Tb/s` -- terabit per second
///
/// # Examples
///
/// ```
/// use bandwidth::Bandwidth;
/// use human_bandwidth::parse_bandwidth;
///
/// assert_eq!(parse_bandwidth("9Tbps 420Gbps"), Ok(Bandwidth::new(9420, 0)));
/// assert_eq!(parse_bandwidth("32Mbps"), Ok(Bandwidth::new(0, 32_000_000)));
/// ```
pub fn parse_bandwidth(s: &str) -> Result<Bandwidth, Error> {
    Parser {
        iter: s.chars(),
        src: s,
        current: (0, 0),
    }
    .parse()
}

/// Formats bandwidth into a human-readable string
///
/// Note: this format is guaranteed to have same value when using
/// parse_bandwidth, but we can change some details of the exact composition
/// of the value.
///
/// # Examples
///
/// ```
/// use bandwidth::Bandwidth;
/// use human_bandwidth::format_bandwidth;
///
/// let val1 = Bandwidth::new(9420, 0);
/// assert_eq!(format_bandwidth(val1).to_string(), "9Tbps 420Gbps");
/// let val2 = Bandwidth::new(0, 32_000_000);
/// assert_eq!(format_bandwidth(val2).to_string(), "32Mbps");
/// ```
pub fn format_bandwidth(val: Bandwidth) -> FormattedBandwidth {
    FormattedBandwidth(val)
}

fn item(f: &mut fmt::Formatter, started: &mut bool, name: &str, value: u32) -> fmt::Result {
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        *started = true;
    }
    Ok(())
}

impl FormattedBandwidth {
    /// Returns a reference to the [`Bandwidth`][] that is being formatted.
    pub fn get_ref(&self) -> &Bandwidth {
        &self.0
    }
}

impl fmt::Display for FormattedBandwidth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let gbps = self.0.as_gbps();
        let bps = self.0.subgbps_bps();

        if gbps == 0 && bps == 0 {
            f.write_str("0bps")?;
            return Ok(());
        }

        let tbps = gbps / 1_000;
        let gbps = gbps % 1_000;

        let mbps = bps / 1_000_000;
        let kbps = bps / 1_000 % 1_000;
        let bps = bps % 1_000;

        let started = &mut false;
        item(f, started, "Tbps", tbps as u32)?;
        item(f, started, "Gbps", gbps as u32)?;
        item(f, started, "Mbps", mbps)?;
        item(f, started, "kbps", kbps)?;
        item(f, started, "bps", bps)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bandwidth::Bandwidth;

    #[test]
    fn test_units() {
        assert_eq!(parse_bandwidth("1bps"), Ok(Bandwidth::new(0, 1)));
        assert_eq!(parse_bandwidth("2bit/s"), Ok(Bandwidth::new(0, 2)));
        assert_eq!(parse_bandwidth("15b/s"), Ok(Bandwidth::new(0, 15)));
        assert_eq!(parse_bandwidth("51kbps"), Ok(Bandwidth::new(0, 51_000)));
        assert_eq!(parse_bandwidth("79Kbps"), Ok(Bandwidth::new(0, 79_000)));
        assert_eq!(parse_bandwidth("81kbit/s"), Ok(Bandwidth::new(0, 81_000)));
        assert_eq!(parse_bandwidth("100Kbit/s"), Ok(Bandwidth::new(0, 100_000)));
        assert_eq!(parse_bandwidth("150kb/s"), Ok(Bandwidth::new(0, 150_000)));
        assert_eq!(parse_bandwidth("410Kb/s"), Ok(Bandwidth::new(0, 410_000)));
        assert_eq!(parse_bandwidth("12Mbps"), Ok(Bandwidth::new(0, 12_000_000)));
        assert_eq!(parse_bandwidth("16mbps"), Ok(Bandwidth::new(0, 16_000_000)));
        assert_eq!(
            parse_bandwidth("24Mbit/s"),
            Ok(Bandwidth::new(0, 24_000_000))
        );
        assert_eq!(
            parse_bandwidth("36mbit/s"),
            Ok(Bandwidth::new(0, 36_000_000))
        );
        assert_eq!(parse_bandwidth("48Mb/s"), Ok(Bandwidth::new(0, 48_000_000)));
        assert_eq!(parse_bandwidth("96mb/s"), Ok(Bandwidth::new(0, 96_000_000)));
        assert_eq!(parse_bandwidth("2Gbps"), Ok(Bandwidth::new(2, 0)));
        assert_eq!(parse_bandwidth("4gbps"), Ok(Bandwidth::new(4, 0)));
        assert_eq!(parse_bandwidth("6Gbit/s"), Ok(Bandwidth::new(6, 0)));
        assert_eq!(parse_bandwidth("8gbit/s"), Ok(Bandwidth::new(8, 0)));
        assert_eq!(parse_bandwidth("16Gb/s"), Ok(Bandwidth::new(16, 0)));
        assert_eq!(parse_bandwidth("40gb/s"), Ok(Bandwidth::new(40, 0)));
        assert_eq!(parse_bandwidth("1Tbps"), Ok(Bandwidth::new(1_000, 0)));
        assert_eq!(parse_bandwidth("2tbps"), Ok(Bandwidth::new(2_000, 0)));
        assert_eq!(parse_bandwidth("4Tbit/s"), Ok(Bandwidth::new(4_000, 0)));
        assert_eq!(parse_bandwidth("8tbit/s"), Ok(Bandwidth::new(8_000, 0)));
        assert_eq!(parse_bandwidth("16Tb/s"), Ok(Bandwidth::new(16_000, 0)));
        assert_eq!(parse_bandwidth("32tb/s"), Ok(Bandwidth::new(32_000, 0)));
    }

    #[test]
    fn test_combo() {
        assert_eq!(
            parse_bandwidth("1bps 2bit/s 3b/s"),
            Ok(Bandwidth::new(0, 6))
        );
        assert_eq!(
            parse_bandwidth("4kbps 5Kbps 6kbit/s"),
            Ok(Bandwidth::new(0, 15_000))
        );
        assert_eq!(
            parse_bandwidth("7Mbps 8mbps 9Mbit/s"),
            Ok(Bandwidth::new(0, 24_000_000))
        );
        assert_eq!(
            parse_bandwidth("10Gbps 11gbps 12Gbit/s"),
            Ok(Bandwidth::new(33, 0))
        );
        assert_eq!(
            parse_bandwidth("13Tbps 14tbps 15Tbit/s"),
            Ok(Bandwidth::new(42_000, 0))
        );
        assert_eq!(
            parse_bandwidth("10Gbps 5Mbps 1b/s"),
            Ok(Bandwidth::new(10, 5_000_001))
        );
        assert_eq!(
            parse_bandwidth("36Mbps 12kbps 24bps"),
            Ok(Bandwidth::new(0, 36_012_024))
        );
    }

    #[test]
    fn test_overflow() {
        assert_eq!(
            parse_bandwidth("100000000000000000000bps"),
            Err(Error::NumberOverflow)
        );
        assert_eq!(
            parse_bandwidth("100000000000000000kbps"),
            Err(Error::NumberOverflow)
        );
        assert_eq!(
            parse_bandwidth("100000000000000Mbps"),
            Err(Error::NumberOverflow)
        );
        assert_eq!(
            parse_bandwidth("100000000000000000000Gbps"),
            Err(Error::NumberOverflow)
        );
        assert_eq!(
            parse_bandwidth("10000000000000000000Tbps"),
            Err(Error::NumberOverflow)
        );
    }

    #[test]
    fn test_nice_error_message() {
        assert_eq!(
            parse_bandwidth("123").unwrap_err().to_string(),
            "bandwidth unit needed, for example 123Mbps or 123bps"
        );
        assert_eq!(
            parse_bandwidth("10 Gbps 1").unwrap_err().to_string(),
            "bandwidth unit needed, for example 1Mbps or 1bps"
        );
        assert_eq!(
            parse_bandwidth("10 byte/s").unwrap_err().to_string(),
            "unknown bandwidth unit \"byte/s\", \
                    supported units: bps, kbps, Mbps, Gbps, Tbps"
        );
    }

    #[test]
    fn test_formatted_bandwidth() {
        assert_eq!(format_bandwidth(Bandwidth::new(0, 0)).to_string(), "0bps");
        assert_eq!(format_bandwidth(Bandwidth::new(0, 1)).to_string(), "1bps");
        assert_eq!(format_bandwidth(Bandwidth::new(0, 15)).to_string(), "15bps");
        assert_eq!(
            format_bandwidth(Bandwidth::new(0, 51_000)).to_string(),
            "51kbps"
        );
        assert_eq!(
            format_bandwidth(Bandwidth::new(0, 32_000_000)).to_string(),
            "32Mbps"
        );
        assert_eq!(
            format_bandwidth(Bandwidth::new(0, 79_000_000)).to_string(),
            "79Mbps"
        );
        assert_eq!(
            format_bandwidth(Bandwidth::new(0, 100_000_000)).to_string(),
            "100Mbps"
        );
        assert_eq!(
            format_bandwidth(Bandwidth::new(0, 150_000_000)).to_string(),
            "150Mbps"
        );
        assert_eq!(
            format_bandwidth(Bandwidth::new(0, 410_000_000)).to_string(),
            "410Mbps"
        );
        assert_eq!(format_bandwidth(Bandwidth::new(1, 0)).to_string(), "1Gbps");
        assert_eq!(
            format_bandwidth(Bandwidth::new(4, 500_000_000)).to_string(),
            "4Gbps 500Mbps"
        );
        assert_eq!(
            format_bandwidth(Bandwidth::new(9420, 0)).to_string(),
            "9Tbps 420Gbps"
        );
    }
}

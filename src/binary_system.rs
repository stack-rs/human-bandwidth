//! Module to allow the display of bandwidth in binary prefix system
//!
//! # Example
//!
//! ```
//! use bandwidth::Bandwidth;
//! use human_bandwidth::binary_system::format_binary_bandwidth;
//!
//! let val = Bandwidth::new(0, 32 * 1024 * 1024);
//! assert_eq!(format_binary_bandwidth(val).to_string(), "4MiB/s");
//! ```

use core::fmt;

use bandwidth::Bandwidth;

#[cfg(feature = "serde")]
pub mod serde;

use crate::{item, Error, OverflowOp, Parser};

/// A wrapper type that allows you to [Display](core::fmt::Display) a [`Bandwidth`] in binary prefix system
#[derive(Debug, Clone)]
pub struct FormattedBinaryBandwidth(Bandwidth);

fn parse_binary_fraction(fraction: u64, fraction_cnt: u32, factore: u64) -> u64 {
    let fraction: f64 = fraction as f64 / (10u64.pow(fraction_cnt)) as f64;
    (fraction * factore as f64).round() as u64
}

impl Parser<'_> {
    fn parse_binary_unit(
        &mut self,
        n: u64,
        fraction: u64,
        fraction_cnt: u32,
        start: usize,
        end: usize,
    ) -> Result<(), Error> {
        let factore = match &self.src[start..end] {
            "Bps" | "Byte/s" | "B/s" | "ops" | "o/s" => 1,
            "kiBps" | "KiBps" | "kiByte/s" | "KiByte/s" | "kiB/s" | "KiB/s" | "kiops" | "Kiops"
            | "kio/s" | "Kio/s" => 1024,
            "MiBps" | "miBps" | "MiByte/s" | "miByte/s" | "MiB/s" | "miB/s" | "Miops" | "miops"
            | "Mio/s" | "mio/s" => 1024 * 1024,
            "GiBps" | "giBps" | "GiByte/s" | "giByte/s" | "GiB/s" | "giB/s" | "Giops" | "giops"
            | "Gio/s" | "gio/s" => 1024_u64.pow(3),
            "TiBps" | "tiBps" | "TiByte/s" | "tiByte/s" | "TiB/s" | "tiB/s" | "Tiops" | "tiops"
            | "Tio/s" | "tio/s" => 1024_u64.pow(4),
            _ => {
                return Err(Error::UnknownBinaryUnit {
                    start,
                    end,
                    unit: self.src[start..end].to_string(),
                    value: n,
                });
            }
        };
        let bps = n
            .mul(factore)?
            .add(parse_binary_fraction(fraction, fraction_cnt, factore))?
            .mul(8)?;
        let (mut gbps, bps) = (bps / 1_000_000_000, bps % 1_000_000_000);
        let mut bps = self.current.1.add(bps)?;
        if bps > 1_000_000_000 {
            gbps = gbps.add(bps / 1_000_000_000)?;
            bps %= 1_000_000_000;
        }
        gbps = self.current.0.add(gbps)?;
        self.current = (gbps, bps);
        Ok(())
    }

    fn parse_binary(mut self) -> Result<Bandwidth, Error> {
        let mut n = self.parse_first_char()?.ok_or(Error::Empty)?;
        let mut decimal = false;
        let mut fraction: u64 = 0;
        let mut fraction_cnt: u32 = 0;
        'outer: loop {
            let mut off = self.off();
            while let Some(c) = self.iter.next() {
                match c {
                    '0'..='9' => {
                        if decimal {
                            if fraction_cnt >= super::FRACTION_PART_LIMIT {
                                continue;
                            }
                            fraction = fraction
                                .checked_mul(10)
                                .and_then(|x| x.checked_add(c as u64 - '0' as u64))
                                .ok_or(Error::NumberOverflow)?;
                            fraction_cnt += 1;
                        } else {
                            n = n
                                .checked_mul(10)
                                .and_then(|x| x.checked_add(c as u64 - '0' as u64))
                                .ok_or(Error::NumberOverflow)?;
                        }
                    }
                    c if c.is_whitespace() => {}
                    '.' => {
                        if decimal {
                            return Err(Error::InvalidCharacter(off));
                        }
                        decimal = true;
                    }
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
                        self.parse_binary_unit(n, fraction, fraction_cnt, start, off)?;
                        n = c as u64 - '0' as u64;
                        fraction = 0;
                        decimal = false;
                        fraction_cnt = 0;
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
            self.parse_binary_unit(n, fraction, fraction_cnt, start, off)?;
            n = match self.parse_first_char()? {
                Some(n) => n,
                None => return Ok(Bandwidth::new(self.current.0, self.current.1 as u32)),
            };
            fraction = 0;
            decimal = false;
            fraction_cnt = 0;
        }
    }
}

/// Parse bandwidth object `1GiBps 12MiBps 5Bps` or `1.012000005GiBps`
///
/// Unlike [`parse_bandwidth`](super::parse_bandwidth), this method expect bandwidth to be written in binary prefix system
///
/// The bandwidth object is a concatenation of rate spans. Where each rate
/// span is an number and a suffix. Supported suffixes:
///
/// * `Bps`, `Byte/s`, `B/s` -- Byte per second
/// * `kiBps`, `kiByte/s`, `kiB/s` -- kiloiByte per second
/// * `MiBps`, `MiByte/s`, `MiB/s` -- megaiByte per second
/// * `GiBps`, `GiByte/s`, `GiB/s` -- gigaiByte per second
/// * `TiBps`, `TiByte/s`, `TiB/s` -- teraiByte per second
///
/// While the number can be integer or decimal, the fractional part less than 1Bps will always be
/// rounded to the closest (ties away from zero).
///
/// # Examples
///
/// ```
/// use bandwidth::Bandwidth;
/// use human_bandwidth::binary_system::parse_binary_bandwidth;
///
/// assert_eq!(parse_binary_bandwidth("9TiBps 420GiBps"), Ok(Bandwidth::new(82772, 609728512)));
/// assert_eq!(parse_binary_bandwidth("4MiBps"), Ok(Bandwidth::new(0, 4 * 8 * 1024 * 1024)));
/// assert_eq!(parse_binary_bandwidth("150.024kiBps"), Ok(Bandwidth::new(0, (150.024 * 1024_f64).round() as u32 * 8)));
/// // The fractional part less than 1Bps will always be ignored
/// assert_eq!(parse_binary_bandwidth("150.02456kiBps"), Ok(Bandwidth::new(0, (150.02456 * 1024_f64).round() as u32 * 8)));
/// ```
pub fn parse_binary_bandwidth(s: &str) -> Result<Bandwidth, Error> {
    Parser {
        iter: s.chars(),
        src: s,
        current: (0, 0),
    }
    .parse_binary()
}

/// Formats bandwidth into a human-readable string using the binary prefix system
///
/// Note: this format is NOT guaranteed to have same value when using
/// parse_binary_bandwidth, the decimal part may varie du to the conversion
/// between binary system and decimal system
///
/// By default it will format the value with the largest possible unit in decimal form.
/// If you want to display integer values only, enable the `display-integer` feature.
///
/// # Examples
///
/// ```
/// use bandwidth::Bandwidth;
/// use human_bandwidth::binary_system::format_binary_bandwidth;
///
/// // Enabling the `display-integer` feature will display integer values only
/// # #[cfg(feature = "display-integer")]
/// # {
/// let val1 = Bandwidth::new(82772, 609728512);
/// assert_eq!(format_binary_bandwidth(val1).to_string(), "9TiB/s 420GiB/s");
/// let val2 = Bandwidth::new(0, 32 * 1024 * 1024);
/// assert_eq!(format_binary_bandwidth(val2).to_string(), "4MiB/s");
/// # }
///
/// // Disabling the `display-integer` feature will display decimal values
/// # #[cfg(not(feature = "display-integer"))]
/// # {
/// let val1 = Bandwidth::from_bps((9 * 1024 + 512) * 1024 * 1024 * 1024 * 8);
/// assert_eq!(format_binary_bandwidth(val1).to_string(), "9.5TiB/s");
/// let val2 = Bandwidth::new(0, 32 * 1024 * 1024);
/// assert_eq!(format_binary_bandwidth(val2).to_string(), "4MiB/s");
/// # }
/// ```
pub fn format_binary_bandwidth(val: Bandwidth) -> FormattedBinaryBandwidth {
    FormattedBinaryBandwidth(val)
}

#[derive(Copy, Clone)]
#[repr(usize)]
enum LargestBinaryUnit {
    Bps = 0,
    KiBps = 1,
    MiBps = 2,
    GiBps = 3,
    TiBps = 4,
}

impl fmt::Display for LargestBinaryUnit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LargestBinaryUnit::Bps => f.write_str("B/s"),
            LargestBinaryUnit::KiBps => f.write_str("kiB/s"),
            LargestBinaryUnit::MiBps => f.write_str("MiB/s"),
            LargestBinaryUnit::GiBps => f.write_str("GiB/s"),
            LargestBinaryUnit::TiBps => f.write_str("TiB/s"),
        }
    }
}

impl FormattedBinaryBandwidth {
    /// Returns a reference to the [`Bandwidth`] that is being formatted.
    pub fn get_ref(&self) -> &Bandwidth {
        &self.0
    }

    /// Enabling the `display-integer` feature will display integer values only
    ///
    /// This method is preserved for backward compatibility and custom formatting.
    pub fn fmt_integer(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let gbps = self.0.as_gbps();
        let bps = self.0.subgbps_bps();

        if gbps == 0 && bps == 0 {
            f.write_str("0B/s")?;
            return Ok(());
        }

        let total: u64 = gbps * 1_000_000_000 + bps as u64;
        let total = (total + 4) / 8;

        let tibps = (total / (1024 * 1024 * 1024 * 1024)) as u32;
        let total = total % (1024 * 1024 * 1024 * 1024);

        let gibps = (total / (1024 * 1024 * 1024)) as u32;
        let total = total % (1024 * 1024 * 1024);

        let mibps = (total / (1024 * 1024)) as u32;
        let total = total % (1024 * 1024);

        let kibps = (total / 1024) as u32;
        let bps = (total % 1024) as u32;

        let started = &mut false;
        item(f, started, "TiB/s", tibps)?;
        item(f, started, "GiB/s", gibps)?;
        item(f, started, "MiB/s", mibps)?;
        item(f, started, "kiB/s", kibps)?;
        item(f, started, "B/s", bps)?;
        Ok(())
    }

    /// Disabling the `display-integer` feature will display decimal values
    ///
    /// This method is preserved for custom formatting.
    pub fn fmt_decimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let gbps = self.0.as_gbps();
        let bps = self.0.subgbps_bps();

        if gbps == 0 && bps == 0 {
            f.write_str("0B/s")?;
            return Ok(());
        }

        let total: u64 = gbps * 1_000_000_000 + bps as u64;
        let total = (total + 4) / 8;

        let tibps = (total / (1024 * 1024 * 1024 * 1024)) as u32;
        let total = total % (1024 * 1024 * 1024 * 1024);

        let gibps = (total / (1024 * 1024 * 1024)) as u32;
        let total = total % (1024 * 1024 * 1024);

        let mibps = (total / (1024 * 1024)) as u32;
        let total = total % (1024 * 1024);

        let kibps = (total / 1024) as u32;
        let bps = (total % 1024) as u32;

        let largest_unit = if tibps > 0 {
            LargestBinaryUnit::TiBps
        } else if gibps > 0 {
            LargestBinaryUnit::GiBps
        } else if mibps > 0 {
            LargestBinaryUnit::MiBps
        } else if kibps > 0 {
            LargestBinaryUnit::KiBps
        } else {
            LargestBinaryUnit::Bps
        };

        let values = [bps, kibps, mibps, gibps, tibps];
        let index = largest_unit as usize;

        let mut value = values[index];

        let mut reminder = 0;
        let mut i = index;
        while i > 0 {
            reminder *= 1024;
            reminder += values[i - 1] as u64;
            i -= 1;
        }
        let mut zeros = index * 3;
        let reminder = reminder as f64 / 1024_u64.pow(index as u32) as f64;
        let mut reminder = (reminder * 1000_u64.pow(index as u32) as f64).round() as u64;
        eprintln!("{value}: {zeros}, {reminder}");
        if let Some(precision) = f.precision() {
            let mut rounding_direction = 0;
            while precision < zeros {
                let loss = reminder % 10;
                reminder /= 10;
                match loss {
                    0 => {
                        // rounding_direction does not change
                    }
                    1..5 => {
                        // we are smaller
                        rounding_direction = -1;
                    }
                    5 => {
                        if rounding_direction == 0 {
                            // we are perfectly in the middle, so we round toward even
                            if reminder % 2 == 1 {
                                reminder += 1;
                                rounding_direction = 1;
                            } else {
                                rounding_direction = -1
                            }
                        } else if rounding_direction == -1 {
                            // we are already smaller than originally
                            // so we go up
                            reminder += 1;
                            rounding_direction = 1;
                        } else {
                            // We were bigger than the original
                            rounding_direction = -1;
                        }
                    }
                    6..10 => {
                        // we are bigger
                        reminder += 1;
                        rounding_direction = 1;
                    }
                    _ => unreachable!("The reminder of a divition by 10 is always between 0 and 9"),
                }
                zeros -= 1;
            }
            if precision == 0 && reminder > 0 {
                value += reminder as u32;
                reminder = 0;
            }
        } else if reminder != 0 {
            while reminder % 10 == 0 {
                reminder /= 10;
                zeros -= 1;
            }
        } else {
            zeros = 0;
        }
        eprintln!("{value}: {zeros}, {reminder}");
        write!(f, "{value}")?;
        if zeros != 0 || reminder != 0 {
            write!(f, ".{reminder:0zeros$}", zeros = zeros)?;
        }
        write!(f, "{}", largest_unit)
    }
}

impl fmt::Display for FormattedBinaryBandwidth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(not(feature = "display-integer"))]
        self.fmt_decimal(f)?;
        #[cfg(feature = "display-integer")]
        self.fmt_integer(f)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bandwidth::Bandwidth;

    fn new_bandwidth(tebi: u16, gibi: u16, mibi: u16, kibi: u16, bytes: u16) -> Bandwidth {
        const KI_B: u64 = 1024 * 8;
        const MI_B: u64 = 1024 * KI_B;
        const GI_B: u64 = 1024 * MI_B;
        const TI_B: u64 = 1024 * GI_B;

        let res: u64 = bytes as u64 * 8
            + kibi as u64 * KI_B
            + mibi as u64 * MI_B
            + gibi as u64 * GI_B
            + tebi as u64 * TI_B;
        Bandwidth::new(res / 1_000_000_000, (res % 1_000_000_000) as u32)
    }

    #[test]
    fn test_units() {
        assert_eq!(
            parse_binary_bandwidth("1Bps"),
            Ok(new_bandwidth(0, 0, 0, 0, 1))
        );
        assert_eq!(
            parse_binary_bandwidth("2Byte/s"),
            Ok(new_bandwidth(0, 0, 0, 0, 2))
        );
        assert_eq!(
            parse_binary_bandwidth("15B/s"),
            Ok(new_bandwidth(0, 0, 0, 0, 15))
        );
        assert_eq!(
            parse_binary_bandwidth("21ops"),
            Ok(new_bandwidth(0, 0, 0, 0, 21))
        );
        assert_eq!(
            parse_binary_bandwidth("22o/s"),
            Ok(new_bandwidth(0, 0, 0, 0, 22))
        );
        assert_eq!(
            parse_binary_bandwidth("51kiBps"),
            Ok(new_bandwidth(0, 0, 0, 51, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("79KiBps"),
            Ok(new_bandwidth(0, 0, 0, 79, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("81kiByte/s"),
            Ok(new_bandwidth(0, 0, 0, 81, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("100KiByte/s"),
            Ok(new_bandwidth(0, 0, 0, 100, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("150kiB/s"),
            Ok(new_bandwidth(0, 0, 0, 150, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("410KiB/s"),
            Ok(new_bandwidth(0, 0, 0, 410, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("251kiops"),
            Ok(new_bandwidth(0, 0, 0, 251, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("279Kiops"),
            Ok(new_bandwidth(0, 0, 0, 279, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("250kio/s"),
            Ok(new_bandwidth(0, 0, 0, 250, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("210Kio/s"),
            Ok(new_bandwidth(0, 0, 0, 210, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("12MiBps"),
            Ok(new_bandwidth(0, 0, 12, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("16miBps"),
            Ok(new_bandwidth(0, 0, 16, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("24MiByte/s"),
            Ok(new_bandwidth(0, 0, 24, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("36miByte/s"),
            Ok(new_bandwidth(0, 0, 36, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("48MiB/s"),
            Ok(new_bandwidth(0, 0, 48, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("96miB/s"),
            Ok(new_bandwidth(0, 0, 96, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("212Miops"),
            Ok(new_bandwidth(0, 0, 212, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("216miops"),
            Ok(new_bandwidth(0, 0, 216, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("248Mio/s"),
            Ok(new_bandwidth(0, 0, 248, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("296mio/s"),
            Ok(new_bandwidth(0, 0, 296, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("2GiBps"),
            Ok(new_bandwidth(0, 2, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("4giBps"),
            Ok(new_bandwidth(0, 4, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("6GiByte/s"),
            Ok(new_bandwidth(0, 6, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("8giByte/s"),
            Ok(new_bandwidth(0, 8, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("16GiB/s"),
            Ok(new_bandwidth(0, 16, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("40giB/s"),
            Ok(new_bandwidth(0, 40, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("202Giops"),
            Ok(new_bandwidth(0, 202, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("204giops"),
            Ok(new_bandwidth(0, 204, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("216Gio/s"),
            Ok(new_bandwidth(0, 216, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("240gio/s"),
            Ok(new_bandwidth(0, 240, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("1TiBps"),
            Ok(new_bandwidth(1, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("2tiBps"),
            Ok(new_bandwidth(2, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("4TiByte/s"),
            Ok(new_bandwidth(4, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("8tiByte/s"),
            Ok(new_bandwidth(8, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("16TiB/s"),
            Ok(new_bandwidth(16, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("32tiB/s"),
            Ok(new_bandwidth(32, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("201Tiops"),
            Ok(new_bandwidth(201, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("202tiops"),
            Ok(new_bandwidth(202, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("216Tio/s"),
            Ok(new_bandwidth(216, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("232tio/s"),
            Ok(new_bandwidth(232, 0, 0, 0, 0))
        );
    }

    #[test]
    fn test_decimal() {
        assert_eq!(
            parse_binary_bandwidth("1.5Bps"),
            Ok(new_bandwidth(0, 0, 0, 0, 2))
        );
        assert_eq!(
            parse_binary_bandwidth("2.5Byte/s"),
            Ok(new_bandwidth(0, 0, 0, 0, 3))
        );
        assert_eq!(
            parse_binary_bandwidth("15.5B/s"),
            Ok(new_bandwidth(0, 0, 0, 0, 16))
        );
        assert_eq!(
            parse_binary_bandwidth("51.6kiBps"),
            Ok(new_bandwidth(0, 0, 0, 51, 614))
        );
        assert_eq!(
            parse_binary_bandwidth("79.78KiBps"),
            Ok(new_bandwidth(0, 0, 0, 79, 799))
        );
        assert_eq!(
            parse_binary_bandwidth("81.923kiByte/s"),
            Ok(new_bandwidth(0, 0, 0, 81, 945))
        );
        assert_eq!(
            parse_binary_bandwidth("100.1234KiByte/s"),
            Ok(new_bandwidth(0, 0, 0, 100, 126))
        );
        assert_eq!(
            parse_binary_bandwidth("150.12345kiB/s"),
            Ok(new_bandwidth(0, 0, 0, 150, 126))
        );
        assert_eq!(
            parse_binary_bandwidth("410.123456KiB/s"),
            Ok(new_bandwidth(0, 0, 0, 410, 126))
        );
        assert_eq!(
            parse_binary_bandwidth("12.123MiBps"),
            Ok(new_bandwidth(0, 0, 12, 125, 975))
        );
        assert_eq!(
            parse_binary_bandwidth("16.1234miBps"),
            Ok(new_bandwidth(0, 0, 16, 126, 370))
        );
        assert_eq!(
            parse_binary_bandwidth("24.12345MiByte/s"),
            Ok(new_bandwidth(0, 0, 24, 126, 423))
        );
        assert_eq!(
            parse_binary_bandwidth("36.123456miByte/s"),
            Ok(new_bandwidth(0, 0, 36, 126, 429))
        );
        assert_eq!(
            parse_binary_bandwidth("48.123MiB/s"),
            Ok(new_bandwidth(0, 0, 48, 125, 975))
        );
        assert_eq!(
            parse_binary_bandwidth("96.1234miB/s"),
            Ok(new_bandwidth(0, 0, 96, 126, 370))
        );
        assert_eq!(
            parse_binary_bandwidth("2.123GiBps"),
            Ok(new_bandwidth(0, 2, 125, 974, 868))
        );
        assert_eq!(
            parse_binary_bandwidth("4.1234giBps"),
            Ok(new_bandwidth(0, 4, 126, 370, 285))
        );
        assert_eq!(
            parse_binary_bandwidth("6.12345GiByte/s"),
            Ok(new_bandwidth(0, 6, 126, 422, 724))
        );
        assert_eq!(
            parse_binary_bandwidth("8.123456giByte/s"),
            Ok(new_bandwidth(0, 8, 126, 428, 1023))
        );
        assert_eq!(
            parse_binary_bandwidth("16.123456789GiB/s"),
            Ok(new_bandwidth(0, 16, 126, 429, 846))
        );
        assert_eq!(
            parse_binary_bandwidth("40.12345678912giB/s"),
            Ok(new_bandwidth(0, 40, 126, 429, 846))
        );
        assert_eq!(
            parse_binary_bandwidth("1.123TiBps"),
            Ok(new_bandwidth(1, 125, 974, 868, 360))
        );
        assert_eq!(
            parse_binary_bandwidth("2.1234tiBps"),
            Ok(new_bandwidth(2, 126, 370, 285, 84))
        );
        assert_eq!(
            parse_binary_bandwidth("4.12345TiByte/s"),
            Ok(new_bandwidth(4, 126, 422, 724, 177))
        );
        assert_eq!(
            parse_binary_bandwidth("8.123456tiByte/s"),
            Ok(new_bandwidth(8, 126, 428, 1022, 639))
        );
        assert_eq!(
            parse_binary_bandwidth("16.123456789TiB/s"),
            Ok(new_bandwidth(16, 126, 429, 845, 825))
        );
        assert_eq!(
            parse_binary_bandwidth("32.12345678912tiB/s"),
            Ok(new_bandwidth(32, 126, 429, 845, 957))
        );
    }

    #[test]
    fn test_combo() {
        assert_eq!(
            parse_binary_bandwidth("1Bps 2Byte/s 3B/s"),
            Ok(new_bandwidth(0, 0, 0, 0, 6))
        );
        assert_eq!(
            parse_binary_bandwidth("4kiBps 5KiBps 6kiByte/s"),
            Ok(new_bandwidth(0, 0, 0, 15, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("7MiBps 8miBps 9MiByte/s"),
            Ok(new_bandwidth(0, 0, 24, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("10GiBps 11giBps 12GiByte/s"),
            Ok(new_bandwidth(0, 33, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("13TiBps 14tiBps 15TiByte/s"),
            Ok(new_bandwidth(42, 0, 0, 0, 0))
        );
        assert_eq!(
            parse_binary_bandwidth("10GiBps 5MiBps 1B/s"),
            Ok(new_bandwidth(0, 10, 5, 0, 1))
        );
        assert_eq!(
            parse_binary_bandwidth("36MiBps 12kiBps 24Bps"),
            Ok(new_bandwidth(0, 0, 36, 12, 24))
        );
    }

    #[test]
    fn test_decimal_combo() {
        assert_eq!(
            parse_binary_bandwidth("1.1Bps 2.2Byte/s 3.3B/s"),
            Ok(new_bandwidth(0, 0, 0, 0, 6))
        );
        assert_eq!(
            parse_binary_bandwidth("4.4kiBps 5.5KiBps 6.6kiByte/s"),
            Ok(new_bandwidth(0, 0, 0, 16, 512))
        );
        assert_eq!(
            parse_binary_bandwidth("7.7MiBps 8.8miBps 9.9MiByte/s"),
            Ok(new_bandwidth(0, 0, 26, 409, 614))
        );
        assert_eq!(
            parse_binary_bandwidth("10.10GiBps 11.11giBps 12.12GiByte/s"),
            Ok(new_bandwidth(0, 33, 337, 942, 82))
        );
        assert_eq!(
            parse_binary_bandwidth("13.13TiBps 14.14tiBps 15.15TiByte/s"),
            Ok(new_bandwidth(42, 430, 81, 942, 82))
        );
        assert_eq!(
            parse_binary_bandwidth("10.1GiBps 5.2MiBps 1.3B/s"),
            Ok(new_bandwidth(0, 10, 107, 614, 410))
        );
        assert_eq!(
            parse_binary_bandwidth("36.1MiBps 12.2kiBps 24.3Bps"),
            Ok(new_bandwidth(0, 0, 36, 114, 639))
        );
    }

    #[test]
    fn test_overflow() {
        assert_eq!(
            parse_binary_bandwidth("100000000000000000000Bps"),
            Err(Error::NumberOverflow)
        );
        assert_eq!(
            parse_binary_bandwidth("100000000000000000kiBps"),
            Err(Error::NumberOverflow)
        );
        assert_eq!(
            parse_binary_bandwidth("100000000000000MiBps"),
            Err(Error::NumberOverflow)
        );
        assert_eq!(
            parse_binary_bandwidth("100000000000000000000GiBps"),
            Err(Error::NumberOverflow)
        );
        assert_eq!(
            parse_binary_bandwidth("10000000000000000000TiBps"),
            Err(Error::NumberOverflow)
        );
    }

    #[test]
    fn test_nice_error_message() {
        assert_eq!(
            parse_binary_bandwidth("123").unwrap_err().to_string(),
            "binary bandwidth unit needed, for example 123MiB/s or 123B/s"
        );
        assert_eq!(
            parse_binary_bandwidth("10 GiBps 1")
                .unwrap_err()
                .to_string(),
            "binary bandwidth unit needed, for example 1MiB/s or 1B/s"
        );
        assert_eq!(
            parse_binary_bandwidth("10 byte/s").unwrap_err().to_string(),
            "unknown binary bandwidth unit \"byte/s\", \
                    supported units: B/s, kiB/s, MiB/s, GiB/s, TiB/s"
        );
    }

    #[test]
    fn test_formatted_bandwidth_integer() {
        struct TestInteger(FormattedBinaryBandwidth);
        impl From<FormattedBinaryBandwidth> for TestInteger {
            fn from(fb: FormattedBinaryBandwidth) -> Self {
                TestInteger(fb)
            }
        }
        impl fmt::Display for TestInteger {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt_integer(f)
            }
        }
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 0, 0, 0))).to_string(),
            "0B/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 0, 0, 1))).to_string(),
            "1B/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 0, 0, 15))).to_string(),
            "15B/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 0, 51, 0))).to_string(),
            "51kiB/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 32, 0, 0))).to_string(),
            "32MiB/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 79, 0, 0))).to_string(),
            "79MiB/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 100, 0, 0))).to_string(),
            "100MiB/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 150, 0, 0))).to_string(),
            "150MiB/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 0, 410, 0, 0))).to_string(),
            "410MiB/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 1, 0, 0, 0))).to_string(),
            "1GiB/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(0, 4, 500, 0, 0))).to_string(),
            "4GiB/s 500MiB/s"
        );
        assert_eq!(
            TestInteger::from(format_binary_bandwidth(new_bandwidth(9, 420, 0, 0, 0))).to_string(),
            "9TiB/s 420GiB/s"
        );
    }

    #[test]
    fn test_formatted_bandwidth_decimal() {
        struct TestDecimal(FormattedBinaryBandwidth);
        impl From<FormattedBinaryBandwidth> for TestDecimal {
            fn from(fb: FormattedBinaryBandwidth) -> Self {
                TestDecimal(fb)
            }
        }
        impl fmt::Display for TestDecimal {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt_decimal(f)
            }
        }
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 0, 0, 0))).to_string(),
            "0B/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 0, 0, 1))).to_string(),
            "1B/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 0, 0, 15))).to_string(),
            "15B/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 0, 51, 256))).to_string(),
            "51.25kiB/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 32, 256, 0))).to_string(),
            "32.25MiB/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 79, 0, 5))).to_string(),
            "79.000005MiB/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 100, 128, 7)))
                .to_string(),
            "100.125007MiB/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 150, 0, 0))).to_string(),
            "150MiB/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 0, 410, 9, 116)))
                .to_string(),
            "410.0089MiB/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 1, 0, 0, 0))).to_string(),
            "1GiB/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(0, 4, 512, 0, 0))).to_string(),
            "4.5GiB/s"
        );
        assert_eq!(
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(8, 768, 0, 0, 0))).to_string(),
            "8.75TiB/s"
        );
        assert_eq!(
            "9.375TiB/s",
            TestDecimal::from(format_binary_bandwidth(new_bandwidth(9, 384, 0, 0, 0))).to_string(),
        );
    }

    #[test]
    fn test_formatted_bandwidth_decimal_with_precision() {
        struct TestDecimal(FormattedBinaryBandwidth);
        impl From<FormattedBinaryBandwidth> for TestDecimal {
            fn from(fb: FormattedBinaryBandwidth) -> Self {
                TestDecimal(fb)
            }
        }
        impl fmt::Display for TestDecimal {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt_decimal(f)
            }
        }
        let bandwidths = [
            (new_bandwidth(0, 0, 0, 0, 0), 0, 0, "B/s", 0),
            (new_bandwidth(0, 0, 0, 0, 1), 1, 0, "B/s", 0),
            (new_bandwidth(0, 0, 0, 0, 15), 15, 0, "B/s", 0),
            (new_bandwidth(0, 0, 0, 51, 256), 51, 250, "kiB/s", 3),
            (new_bandwidth(0, 0, 32, 256, 0), 32, 250_000, "MiB/s", 6),
            (new_bandwidth(0, 0, 79, 0, 5), 79, 5, "MiB/s", 6),
            (new_bandwidth(0, 0, 100, 128, 7), 100, 125_007, "MiB/s", 6),
            (new_bandwidth(0, 0, 150, 0, 0), 150, 0, "MiB/s", 6),
            (new_bandwidth(0, 0, 410, 9, 116), 410, 8_900, "MiB/s", 6),
            (new_bandwidth(0, 1, 0, 0, 0), 1, 0, "GiB/s", 9),
            (new_bandwidth(0, 4, 512, 0, 0), 4, 500_000_000, "GiB/s", 9),
            (
                new_bandwidth(8, 768, 0, 0, 0),
                8,
                750_000_000_000,
                "TiB/s",
                12,
            ),
            (
                new_bandwidth(9, 384, 0, 0, 0),
                9,
                375_000_000_000,
                "TiB/s",
                12,
            ),
        ];
        for precision in 0..7 {
            println!("{precision}");
            for (bandwidth, int, fract, unit, max_precision) in bandwidths.iter() {
                let bandwidth = TestDecimal::from(format_binary_bandwidth(*bandwidth));
                let pow = 10_u64.pow((max_precision - precision.min(*max_precision)) as u32);
                let fract = if pow != 1 {
                    if fract % pow > pow / 2 || fract % pow == pow / 2 && fract / pow % 2 == 1 {
                        fract / pow + 1
                    } else {
                        fract / pow
                    }
                } else {
                    *fract
                };
                println!("{fract}");
                println!("{int}.{fract}");
                if precision != 0 && *max_precision != 0 {
                    assert_eq!(
                        format!("{bandwidth:.precision$}"),
                        format!(
                            "{int}.{fract:0precision$}{unit}",
                            precision = precision.min(*max_precision)
                        )
                    );
                } else {
                    let int = if fract == 1 { int + 1 } else { *int };
                    assert_eq!(format!("{bandwidth:.precision$}"), format!("{int}{unit}"));
                }
            }
        }
    }
}

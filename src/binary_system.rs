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

use std::fmt;

use bandwidth::Bandwidth;

use crate::item;

/// A wrapper type that allows you to [Display](std::fmt::Display) a [`Bandwidth`] in binary prefix system
#[derive(Debug, Clone)]
pub struct FormattedBinaryBandwidth(Bandwidth);

/// Formats bandwidth into a human-readable string using the binary prefix system
///
// / TODO: Parsing from binary prefix system
// / Note: this format is guaranteed to have same value when using
// / parse_bandwidth, but we can change some details of the exact composition
// / of the value.
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
/// let val1 = Bandwidth::new(82772, 609728512);
/// assert_eq!(format_binary_bandwidth(val1).to_string(), "9.42TiB/s");
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

        let total: u128 = gbps as u128 * 1_000_000_000 + bps as u128;
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

        let total: u128 = gbps as u128 * 1_000_000_000 + bps as u128;
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

        write!(f, "{}", values[index])?;

        let mut reminder = 0;
        let mut i = index;
        while i > 0 {
            reminder *= 1024;
            reminder += values[i - 1] as u128;
            i -= 1;
        }
        let mut zeros = index * 3;
        let reminder = reminder as f64 / 1024_u128.pow(index as u32) as f64;
        let mut reminder = (reminder * 1000_u128.pow(index as u32) as f64).round() as u128;
        if reminder != 0 {
            while reminder % 10 == 0 {
                reminder /= 10;
                zeros -= 1;
            }
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
        const KI_B: u128 = 1024 * 8;
        const MI_B: u128 = 1024 * KI_B;
        const GI_B: u128 = 1024 * MI_B;
        const TI_B: u128 = 1024 * GI_B;

        let res: u128 = bytes as u128 * 8
            + kibi as u128 * KI_B
            + mibi as u128 * MI_B
            + gibi as u128 * GI_B
            + tebi as u128 * TI_B;
        Bandwidth::new((res / 1_000_000_000) as u64, (res % 1_000_000_000) as u32)
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
}

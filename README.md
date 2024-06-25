# Human Bandwidth

[![github-repo](https://img.shields.io/badge/github-stack--rs/human--bandwidth-f5dc23?logo=github)](https://github.com/stack-rs/human-bandwidth)
[![crates.io](https://img.shields.io/crates/v/human--bandwidth.svg?logo=rust)](https://crates.io/crates/human-bandwidth)
[![docs.rs](https://img.shields.io/badge/docs.rs-human--bandwidth-blue?logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K)](https://docs.rs/human-bandwidth)
[![LICENSE Apache-2.0](https://img.shields.io/github/license/stack-rs/human--bandwidth?logo=Apache)](https://github.com/stack-rs/human-bandwidth/blob/main/LICENSE)

A library providing human-readable format parsing and formating for [bandwidth](https://crates.io/crates/bandwidth). Enable `serde` feature for serde integration.

**MSRV**: 1.60

## Examples

More detailed usage can be found on [documentation](https://docs.rs/human-bandwidth).

For parsing and formating:

```rust
use bandwidth::Bandwidth;
use human_bandwidth::Bandwidth;

fn main() {
    // Parse bandwidth from human-readable string
    assert_eq!(parse_bandwidth("9Tbps 420Gbps"), Ok(Bandwidth::new(9420, 0)));
    assert_eq!(parse_bandwidth("32Mbps"), Ok(Bandwidth::new(0, 32_000_000)));

    // Format bandwidth to human-readable string
    let val1 = Bandwidth::new(9420, 0);
    assert_eq!(format_bandwidth(val1).to_string(), "9Tbps 420Gbps");
    let val2 = Bandwidth::new(0, 32_000_000);
    assert_eq!(format_bandwidth(val2).to_string(), "32Mbps");
}
```

To integrate with `serde`:

```rust
use serde::{Serialize, Deserialize};
use bandwidth::Bandwidth;

#[derive(Serialize, Deserialize)]
struct Foo {
    #[serde(with = "human_bandwidth::serde")]
    bandwidth: Bandwidth,
}

fn main () {
    let json = r#"{"bandwidth": "1kbps"}"#;
    let foo = serde_json::from_str::<Foo>(json).unwrap();
    assert_eq!(foo.bandwidth, Bandwidth::from_kbps(1));
    let reverse = serde_json::to_string(&foo).unwrap();
    assert_eq!(reverse, r#"{"bandwidth":"1kbps"}"#)
}
```

## Maintainer

[@BobAnkh](https://github.com/BobAnkh)

## How to contribute

You should follow our [Code of Conduct](/CODE_OF_CONDUCT.md).

See [CONTRIBUTING GUIDELINES](/CONTRIBUTING.md) for contributing conventions.

Make sure to pass all the tests before submitting your code.

### Contributors

## LICENSE

[Apache-2.0](LICENSE) © stack-rs

## Credits

- [humantime](https://github.com/tailhook/humantime)
- [humantime-serde](https://github.com/jean-airoldie/humantime-serde)

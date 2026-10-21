#!/usr/bin/env python3
"""Refresh defaults without silently removing existing libvips declarations."""
import argparse
import re
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("generated", type=Path, help="exact binding.rs printed by build.rs")
    parser.add_argument("--version", required=True, help="expected libvips header version")
    args = parser.parse_args()
    destination = Path(__file__).resolve().parents[1] / "src/bindings/prebuilt.rs"
    source = args.generated.read_text()
    version = re.search(r'pub const VIPS_VERSION:.*?b"([^"\\]+)\\0"', source)
    if not version or version.group(1) != args.version:
        parser.error("generated header version does not match --version")
    for alias, scalar in [("gsize", "usize"), ("gssize", "isize")]:
        if f"pub type {alias} = {scalar};" not in source:
            parser.error(f"{alias} must be normalized to {scalar}")
    symbols = lambda text: set(re.findall(r"pub fn (vips_\w+)\(", text))
    old, new = symbols(destination.read_text()), symbols(source)
    if old - new:
        parser.error("refresh would remove existing APIs: " + ", ".join(sorted(old - new)))
    # This file describes a reusable API surface. A maintainer's optional codec
    # configuration is not the configuration of each consumer's linked library.
    source, count = re.subn(
        r'pub const VIPS_CONFIG\s*:\s*&\s*\[u8\s*;\s*\d+\]\s*=\s*b"(?:\\.|[^"\\])*"\s*;',
        lambda _: 'pub const VIPS_CONFIG: &[u8; 29] = b"vendored headers for bindgen\\0";',
        source,
    )
    if count != 1:
        parser.error("expected exactly one VIPS_CONFIG declaration")
    destination.write_text(source)
    print(f"libvips {args.version}: {len(new)} declarations; {len(new - old)} added, none removed")


if __name__ == "__main__":
    main()

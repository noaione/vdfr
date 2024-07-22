# vdfr

A Rust library for reading Valve's binary KeyValue format.

## Supported format
- App Info v27, 28, and 29.
- Package Info
- Standard binary keyvalues.

The exposed public APIs from `vdfr` crate is:
- `parse_app_info` (for AppInfo)
- `parse_package_info` (for PackageInfo)
- `parse_keyvalues` for standard binary key values.

There's two implementation:
- `legacy_parser`, the original one created by drguildo with `byteorder` crate
- `parser`, the new one created by noaione with `nom` crate.

There's some significant improvement with `nom`, but it might be possible to make it faster?

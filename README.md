# vdfr

A Rust library for reading Valve's binary KeyValue format.

## Supported format
- App Info v27, 28, and 29.
- Package Info
- Standard binary keyvalues.

## Usage
### API Usage
The exposed public APIs from `vdfr` crate is:
- `parse_app_info` (for AppInfo)
- `parse_package_info` (for PackageInfo)
- `parse_keyvalues` for standard binary key values.

There's two implementation:
- `legacy_parser`, the original one created by drguildo with `byteorder` crate
- `parser`, the new one created by noaione with `nom` crate.

There's some significant improvement with `nom`, but it might be possible to make it faster?

### CLI usage
First thing first, build the project first:
1. `cargo build --all --release`
2. `.\target\release\vdf.exe` or `./target/release/vdf`

```bash
Usage: vdf <COMMAND>

Commands:
  app   Parse app info file
  pkg   Parse package info file
  kv    Parse key-values file
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

Example to parse appinfo.vdf:
```
$ vdf app appinfo.vdf
```

Redump data into JSON file:
```
$ vdf app appinfo.vdf --redump
```

Use the legacy parser:
```
$ vdf app appinfo.vdf --legacy
```

Same with appinfo/app, packageinfo/pkg, and keyvalues/kv has same parameters.

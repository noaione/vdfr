//! Writer for the VDF binary format.

use std::collections::HashSet;

use crate::{
    common::KeyValues, App, AppInfo, AppInfoVersion, Package, PackageInfo, Value, BIN_END,
};

enum KeyFormat {
    // v29 format with string pools
    Index(u32),
    // <v29 format
    String(String),
}

fn write_utf8<W: std::io::Write>(writer: &mut W, string: &str) -> std::io::Result<()> {
    writer.write_all(string.as_bytes())?;
    // Null terminator
    writer.write_all(&[0])
}

/// Write a UTF-16 string (wide string) to the writer.
/// Uses little-endian encoding.
fn write_utf16<W: std::io::Write>(writer: &mut W, string: &str) -> std::io::Result<()> {
    for c in string.encode_utf16() {
        writer.write_all(&c.to_le_bytes())?;
    }
    // There's 2 bytes for the null terminator + 1 extra byte
    writer.write_all(&[0, 0, 0])
}

fn write_keyvalue<W: std::io::Write>(
    writer: &mut W,
    key: KeyFormat,
    value: &Value,
    string_pools: &mut HashSet<String>,
) -> std::io::Result<()> {
    // Write the bin format
    value.save_bin(writer)?;

    // Write key
    match key {
        KeyFormat::Index(index) => {
            writer.write_all(&index.to_le_bytes())?;
        }
        KeyFormat::String(string) => {
            // Use utf8 with NULL terminator
            write_utf8(writer, &string)?;
        }
    }

    // Write value
    match value {
        Value::StringType(string) => {
            write_utf8(writer, string)?;
        }
        Value::WideStringType(string) => {
            write_utf16(writer, string)?;
        }
        Value::Int32Type(i) | Value::PointerType(i) | Value::ColorType(i) => {
            writer.write_all(&i.to_le_bytes())?;
        }
        Value::UInt64Type(ui) => {
            writer.write_all(&ui.to_le_bytes())?;
        }
        Value::Int64Type(i) => {
            writer.write_all(&i.to_le_bytes())?;
        }
        Value::Float32Type(f) => {
            writer.write_all(&f.to_le_bytes())?;
        }
        Value::KeyValueType(kv) => {
            write_keyvalues_internal(writer, kv, string_pools)?;
            // writer.write_all(&[BIN_END])?;
        }
        Value::ArrayType(array) => {
            // Array is our custom type, it's parsed back into KeyValues like:
            // "key" { "0" "value" "1" "value" }
            // So we need to write it as a KeyValues
            let keymaps: KeyValues = array
                .iter()
                .enumerate()
                .map(|(idx, kv_arr)| {
                    let key = idx.to_string();
                    (key, kv_arr.clone())
                })
                .collect();
            write_keyvalues_internal(writer, &keymaps, string_pools)?;
        }
    }

    Ok(())
}

fn find_key_index(key: &str, string_pools: &mut HashSet<String>) -> Option<u32> {
    string_pools
        .iter()
        .enumerate()
        .filter_map(
            |(idx, name)| {
                if name == key {
                    Some(idx as u32)
                } else {
                    None
                }
            },
        )
        .next()
}

fn write_keyvalues_internal<W: std::io::Write>(
    writer: &mut W,
    keyvalues: &KeyValues,
    string_pools: &mut HashSet<String>,
) -> std::io::Result<()> {
    for (key, value) in keyvalues {
        let key_data = if string_pools.is_empty() {
            KeyFormat::String(key.clone())
        } else {
            let key_idx = find_key_index(key, string_pools).unwrap();
            KeyFormat::Index(key_idx)
        };

        write_keyvalue(writer, key_data, value, string_pools)?;
    }
    writer.write_all(&[BIN_END])?;

    Ok(())
}

pub fn write_keyvalues<W: std::io::Write>(
    writer: &mut W,
    keyvalues: &KeyValues,
) -> std::io::Result<()> {
    write_keyvalues_internal(writer, keyvalues, &mut HashSet::new())
}

fn collect_string_pools_from_value(string_pools: &mut HashSet<String>, value: &Value) {
    match value {
        Value::KeyValueType(kv) => {
            collect_string_pools(string_pools, kv);
        }
        Value::ArrayType(array) => {
            for (key, value) in array.iter().enumerate() {
                let key = key.to_string();
                string_pools.insert(key.clone());
                collect_string_pools_from_value(string_pools, value);
            }
        }
        _ => {}
    }
}

pub fn collect_string_pools(string_pools: &mut HashSet<String>, key_values: &KeyValues) {
    for (key, value) in key_values {
        string_pools.insert(key.clone());
        collect_string_pools_from_value(string_pools, value);
    }
}

fn write_app<W: std::io::Write + std::io::Seek>(
    writer: &mut W,
    app: &App,
    string_pools: &mut HashSet<String>,
) -> std::io::Result<()> {
    // Write the app info
    writer.write_all(&app.id.to_le_bytes())?;
    writer.write_all(&app.size.to_le_bytes())?;
    writer.write_all(&app.state.to_le_bytes())?;
    writer.write_all(&app.last_update.to_le_bytes())?;
    writer.write_all(&app.access_token.to_le_bytes())?;
    writer.write_all(&*app.checksum_txt)?;
    writer.write_all(&app.change_number.to_le_bytes())?;
    if let Some(checksum_bin) = &app.checksum_bin {
        writer.write_all(checksum_bin.as_bytes())?;
    }

    write_keyvalues_internal(writer, &app.key_values, string_pools)
}

pub fn write_app_info<W: std::io::Write + std::io::Seek>(
    writer: &mut W,
    app_info: &AppInfo,
) -> std::io::Result<()> {
    // Write the app info
    let version_magic: u32 = app_info.version.into();
    writer.write_all(&version_magic.to_le_bytes())?;
    // Write universe
    writer.write_all(&app_info.universe.to_le_bytes())?;

    // If v29, let's do the string pools
    let mut string_pools = HashSet::new();
    let offset_back = if app_info.version == AppInfoVersion::V29 {
        app_info.apps.iter().for_each(|(_, app)| {
            collect_string_pools(&mut string_pools, &app.key_values);
        });

        // Temporarily write the offset and size of the string pools
        let current_pos = writer.seek(std::io::SeekFrom::Current(0))?;
        let temp = 0i64;
        writer.write(&temp.to_le_bytes())?;
        Some(current_pos)
    } else {
        None
    };

    for (_, app) in &app_info.apps {
        write_app(writer, app, &mut string_pools)?;
    }

    // Get the current position, this is what we write later back in the offset
    let current_pos = writer.seek(std::io::SeekFrom::Current(0))?;

    // Write the string pools first
    writer.write(&string_pools.len().to_le_bytes())?;
    for string in string_pools {
        write_utf8(writer, &string)?;
    }

    // Write the offset back
    if let Some(offset) = offset_back {
        writer.seek(std::io::SeekFrom::Start(offset))?;
        writer.write(&(current_pos as i64).to_le_bytes())?;
    }

    Ok(())
}

fn write_package<W: std::io::Write>(writer: &mut W, package_info: &Package) -> std::io::Result<()> {
    // Write the package
    writer.write_all(&package_info.id.to_le_bytes())?;
    writer.write_all(&*package_info.checksum)?;
    writer.write_all(&package_info.change_number.to_le_bytes())?;
    writer.write_all(&package_info.pics.to_le_bytes())?;

    write_keyvalues_internal(writer, &package_info.key_values, &mut HashSet::new())
}

pub fn write_package_info<W: std::io::Write>(
    writer: &mut W,
    package_info: &PackageInfo,
) -> std::io::Result<()> {
    // Write the package info
    writer.write_all(&package_info.version.to_le_bytes())?;
    writer.write_all(&package_info.universe.to_le_bytes())?;
    for (_, package) in &package_info.packages {
        write_package(writer, package)?;
    }

    Ok(())
}

use std::{collections::BTreeMap, io::Error};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::{
    common::{
        map_keyvalues_sequence, App, AppInfo, KeyValueOptions, KeyValues, Package, PackageInfo,
        Value, VdfrError, BIN_COLOR, BIN_END, BIN_END_ALT, BIN_FLOAT32, BIN_INT32, BIN_INT64,
        BIN_KV, BIN_POINTER, BIN_STRING, BIN_UINT64, BIN_WIDESTRING,
    },
    AppInfoVersion, SHA1,
};

pub fn parse_app_info<R: std::io::Read + std::io::Seek>(
    reader: &mut R,
) -> Result<AppInfo, VdfrError> {
    let version: AppInfoVersion = reader.read_u32::<LittleEndian>()?.try_into()?;

    let universe = reader.read_u32::<LittleEndian>()?;

    let mut options = KeyValueOptions::default();

    if version == AppInfoVersion::V29 {
        let offset_table = reader.read_i64::<LittleEndian>()?;
        let old_offset = reader.stream_position()?.clone();
        reader.seek(std::io::SeekFrom::Start(offset_table as u64))?;
        let string_count = reader.read_u32::<LittleEndian>()?;
        options.string_pool = (0..string_count)
            .map(|_| read_string(reader, false).unwrap())
            .collect();
        reader.seek(std::io::SeekFrom::Start(old_offset))?;
    }

    let mut appinfo = AppInfo {
        universe,
        version,
        apps: BTreeMap::new(),
    };

    loop {
        let app_id = reader.read_u32::<LittleEndian>()?;
        if app_id == 0 {
            break;
        }

        let size = reader.read_u32::<LittleEndian>()?;
        let state = reader.read_u32::<LittleEndian>()?;
        let last_update = reader.read_u32::<LittleEndian>()?;
        let access_token = reader.read_u64::<LittleEndian>()?;

        let mut checksum_txt: [u8; 20] = [0; 20];
        reader.read_exact(&mut checksum_txt)?;

        let change_number = reader.read_u32::<LittleEndian>()?;

        let checksum_bin = match version {
            // Skip checksum_bin for v27
            AppInfoVersion::V27 => None,
            _ => {
                let mut checksum_bin: [u8; 20] = [0; 20];
                reader.read_exact(&mut checksum_bin)?;
                Some(checksum_bin)
            }
        };

        let key_values = parse_keyvalues(reader, options.clone())?;
        let key_values = map_keyvalues_sequence(&key_values);

        let app = App {
            id: app_id,
            size,
            state,
            last_update,
            access_token,
            checksum_txt: SHA1::new(checksum_txt),
            checksum_bin: checksum_bin.map(SHA1::new),
            change_number,
            key_values,
        };
        appinfo.apps.insert(app_id, app);
    }

    Ok(appinfo)
}

pub fn parse_package_info<R: std::io::Read>(reader: &mut R) -> Result<PackageInfo, VdfrError> {
    let version = reader.read_u32::<LittleEndian>()?;
    let universe = reader.read_u32::<LittleEndian>()?;

    let mut packageinfo = PackageInfo {
        version,
        universe,
        packages: BTreeMap::new(),
    };

    loop {
        let package_id = reader.read_u32::<LittleEndian>()?;

        if package_id == 0xffffffff {
            break;
        }

        let mut checksum: [u8; 20] = [0; 20];
        reader.read_exact(&mut checksum)?;

        let change_number = reader.read_u32::<LittleEndian>()?;

        // XXX: No idea what this is. Seems to get ignored in vdf.py.
        let pics = reader.read_u64::<LittleEndian>()?;

        let key_values = parse_keyvalues(reader, KeyValueOptions::default())?;
        let key_values = map_keyvalues_sequence(&key_values);

        let package = Package {
            id: package_id,
            checksum: SHA1::new(checksum),
            change_number,
            pics,
            key_values,
        };

        packageinfo.packages.insert(package_id, package);
    }

    Ok(packageinfo)
}

pub fn parse_keyvalues<R: std::io::Read>(
    reader: &mut R,
    options: KeyValueOptions,
) -> Result<KeyValues, VdfrError> {
    let current_bin_end = if options.alt_format {
        BIN_END_ALT
    } else {
        BIN_END
    };

    let mut node = KeyValues::new();

    loop {
        let t = reader.read_u8()?;
        if t == current_bin_end {
            return Ok(node);
        }

        let key = if options.string_pool.is_empty() {
            read_string(reader, false)?
        } else {
            let idx = reader.read_u32::<LittleEndian>()? as usize;
            options.string_pool[idx].clone()
        };

        if t == BIN_KV {
            let subnode = parse_keyvalues(reader, options.clone())?;
            node.insert(key, Value::KeyValueType(subnode));
        } else if t == BIN_STRING {
            let s = read_string(reader, false)?;
            node.insert(key, Value::StringType(s));
        } else if t == BIN_WIDESTRING {
            let s = read_string(reader, true)?;
            node.insert(key, Value::WideStringType(s));
        } else if [BIN_INT32, BIN_POINTER, BIN_COLOR].contains(&t) {
            let val = reader.read_i32::<LittleEndian>()?;
            if t == BIN_INT32 {
                node.insert(key, Value::Int32Type(val));
            } else if t == BIN_POINTER {
                node.insert(key, Value::PointerType(val));
            } else if t == BIN_COLOR {
                node.insert(key, Value::ColorType(val));
            }
        } else if t == BIN_UINT64 {
            let val = reader.read_u64::<LittleEndian>()?;
            node.insert(key, Value::UInt64Type(val));
        } else if t == BIN_INT64 {
            let val = reader.read_i64::<LittleEndian>()?;
            node.insert(key, Value::Int64Type(val));
        } else if t == BIN_FLOAT32 {
            let val = reader.read_f32::<LittleEndian>()?;
            node.insert(key, Value::Float32Type(val));
        } else {
            return Err(VdfrError::InvalidType(t));
        }
    }
}

fn read_string<R: std::io::Read>(reader: &mut R, wide: bool) -> Result<String, Error> {
    if wide {
        let mut buf: Vec<u16> = vec![];
        loop {
            // Maybe this should be big-endian?
            let c = reader.read_u16::<LittleEndian>()?;
            if c == 0 {
                break;
            }
            buf.push(c);
        }
        return Ok(std::string::String::from_utf16_lossy(&buf).to_string());
    } else {
        let mut buf: Vec<u8> = vec![];
        loop {
            let c = reader.read_u8()?;
            if c == 0 {
                break;
            }
            buf.push(c);
        }
        return Ok(std::string::String::from_utf8_lossy(&buf).to_string());
    }
}

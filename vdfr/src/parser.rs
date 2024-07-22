use std::collections::BTreeMap;

use nom::{
    bytes::complete::{take, take_until},
    error::{ErrorKind, ParseError},
    multi::{count, many0},
    number::complete::{be_u16, le_f32, le_i32, le_i64, le_u16, le_u32, le_u64, le_u8},
    sequence::tuple,
    IResult,
};

use crate::{
    common::{
        map_keyvalues_sequence, App, AppInfo, KeyValueOptions, KeyValues, Value, VdfrError,
        BIN_COLOR, BIN_END, BIN_END_ALT, BIN_FLOAT32, BIN_INT32, BIN_INT64, BIN_KV, BIN_POINTER,
        BIN_STRING, BIN_UINT64, BIN_WIDESTRING,
    },
    AppInfoVersion, Package, PackageInfo, SHA1,
};

fn throw_nom_error(error: nom::Err<nom::error::Error<&[u8]>>) -> VdfrError {
    // clone the error to avoid lifetime issues
    match &error {
        nom::Err::Error(e) | nom::Err::Failure(e) => {
            // get like 64 bytes of data to show in the error message
            let str_data = if let nom::Err::Error(_) = &error {
                "Error"
            } else {
                "Failure"
            };
            let data = e.input;
            let data = if data.len() > 64 { &data[..64] } else { data };

            VdfrError::NomError(format!("{}: {:?}, data: {:?}", str_data, e.code, data))
        }
        nom::Err::Incomplete(e) => {
            let need_amount = if let nom::Needed::Size(amount) = e {
                format!("{} bytes", amount)
            } else {
                "unknown amount".to_string()
            };

            VdfrError::NomError(format!("Incomplete data, need: {}", need_amount))
        }
    }
}

struct VdfrNomError {
    message: String,
}

fn format_data(data: &[u8]) -> &[u8] {
    if data.len() > 64 {
        &data[..64]
    } else {
        data
    }
}

impl ParseError<&[u8]> for VdfrNomError {
    fn from_error_kind(input: &[u8], kind: nom::error::ErrorKind) -> Self {
        VdfrNomError {
            message: format!("Error: {:?}, data: {:?}", kind, format_data(input)),
        }
    }

    // if combining multiple errors, we show them one after the other
    fn append(input: &[u8], kind: ErrorKind, other: Self) -> Self {
        let message = format!("{}{:?}:\t{:?}\n", other.message, kind, format_data(input));
        println!("{}", message);
        VdfrNomError { message }
    }

    fn from_char(input: &[u8], c: char) -> Self {
        let message = format!("'{}':\t{:?}\n", c, format_data(input));
        println!("{}", message);
        VdfrNomError { message }
    }

    fn or(self, other: Self) -> Self {
        let message = format!("{}\tOR\n{}\n", self.message, other.message);
        println!("{}", message);
        VdfrNomError { message }
    }
}

impl VdfrNomError {
    fn with_message(&self, input: &str) -> Self {
        VdfrNomError {
            message: format!("{}: {}", input, self.message),
        }
    }
}

fn throw_nom_custom_error(error: nom::Err<VdfrNomError>) -> VdfrError {
    match error {
        nom::Err::Error(e) | nom::Err::Failure(e) => VdfrError::NomError(e.message),
        nom::Err::Incomplete(e) => {
            let need_amount = if let nom::Needed::Size(amount) = e {
                format!("{} bytes", amount)
            } else {
                "unknown amount".to_string()
            };

            VdfrError::NomError(format!("Incomplete data, need: {}", need_amount))
        }
    }
}

pub fn parse_app_info(data: &[u8]) -> Result<AppInfo, VdfrError> {
    let (data, (version, universe)) = tuple((le_u32, le_u32))(data).map_err(throw_nom_error)?;
    let version: AppInfoVersion = version.try_into()?;

    let (payloads, options) = match version {
        AppInfoVersion::V27 | AppInfoVersion::V28 => (data, KeyValueOptions::default()),
        AppInfoVersion::V29 => {
            let (data, offset) = le_i64(data).map_err(throw_nom_error)?;

            // Use nom to jump to offset_table and read the string pool
            // data is the remaining data after reading version, universe, and offset.
            // to ensure we actually jump to the offset, we need to subtract the amount of data read so far.
            let read_amount = 4usize + 4 + 8;
            let offset_actual = (offset as usize) - read_amount;
            // Left side, is the remainder which is the string pools, while payload is the actual data.
            let (string_pools, payload) = take(offset_actual)(data).map_err(throw_nom_error)?;
            let (string_pools, count) = le_u32(string_pools).map_err(throw_nom_error)?;

            let (_, string_pool) =
                read_string_pools(string_pools, count as usize).map_err(throw_nom_custom_error)?;

            (
                payload,
                KeyValueOptions {
                    string_pool,
                    alt_format: false,
                },
            )
        }
    };

    let (_, mut apps) = parse_apps(payloads, &options, &version).map_err(throw_nom_custom_error)?;

    // Remove the empty app (0)
    apps.remove(&0);

    Ok(AppInfo {
        version,
        universe,
        apps,
    })
}

fn parse_apps<'a>(
    data: &'a [u8],
    options: &'a KeyValueOptions,
    version: &'a AppInfoVersion,
) -> IResult<&'a [u8], BTreeMap<u32, App>, VdfrNomError> {
    let (rest, apps) = many0(|d| parse_app(d, options, version))(data)?;

    let hash_apps: BTreeMap<u32, App> = apps.into_iter().map(|app| (app.id, app)).collect();

    Ok((rest, hash_apps))
}

fn parse_app<'a>(
    data: &'a [u8],
    options: &'a KeyValueOptions,
    version: &'a AppInfoVersion,
) -> IResult<&'a [u8], App, VdfrNomError> {
    let (data, app_id) = le_u32(data)?;

    if app_id == 0 {
        // End of apps, return empty app
        Ok((
            data,
            App {
                id: 0,
                size: 0,
                state: 0,
                last_update: 0,
                access_token: 0,
                checksum_txt: SHA1::default(),
                checksum_bin: Some(SHA1::default()),
                change_number: 0,
                key_values: BTreeMap::new(),
            },
        ))
    } else {
        let (data, (size, state, last_update, access_token)) =
            tuple((le_u32, le_u32, le_u32, le_u64))(data)?;

        let (data, checksum_txt) = take(20usize)(data)?;
        let (data, change_number) = le_u32(data)?;
        let (data, checksum_bin) = match version {
            AppInfoVersion::V27 => {
                // we skip checksum_bin
                (data, None)
            }
            _ => {
                let (data, checksum_bin) = take(20usize)(data)?;
                (data, Some(SHA1::new(checksum_bin.try_into().unwrap())))
            }
        };

        let (data, key_values) = parse_bytes_kv(data, options)?;
        let key_values = map_keyvalues_sequence(&key_values);

        Ok((
            data,
            App {
                id: app_id,
                size,
                state,
                last_update,
                access_token,
                checksum_txt: SHA1::new(checksum_txt.try_into().unwrap()),
                checksum_bin,
                change_number,
                key_values,
            },
        ))
    }
}

pub fn parse_package_info(data: &[u8]) -> Result<PackageInfo, VdfrError> {
    let (data, (version, universe)) = tuple((le_u32, le_u32))(data).map_err(throw_nom_error)?;

    let (_, mut packages) =
        parse_packages(data, &KeyValueOptions::default()).map_err(throw_nom_custom_error)?;

    packages.remove(&0xffffffff); // Remove the empty package (0xffffffff

    Ok(PackageInfo {
        version,
        universe,
        packages,
    })
}

fn parse_packages<'a>(
    data: &'a [u8],
    options: &'a KeyValueOptions,
) -> IResult<&'a [u8], BTreeMap<u32, Package>, VdfrNomError> {
    let (rest, packages) = many0(|d| parse_package(d, options))(data)?;

    let hash_packages: BTreeMap<u32, Package> =
        packages.into_iter().map(|app| (app.id, app)).collect();

    Ok((rest, hash_packages))
}

fn parse_package<'a>(
    data: &'a [u8],
    options: &'a KeyValueOptions,
) -> IResult<&'a [u8], Package, VdfrNomError> {
    let (data, package_id) = le_u32(data)?;
    if package_id == 0xffffffff {
        return Ok((
            data,
            Package {
                id: 0xffffffff,
                checksum: SHA1::default(),
                change_number: 0,
                pics: 0,
                key_values: BTreeMap::new(),
            },
        ));
    }

    let (data, checksum) = take(20usize)(data)?;
    let (data, (change_number, pics)) = tuple((le_u32, le_u64))(data)?;

    let (data, key_values) = parse_bytes_kv(data, options)?;
    let key_values = map_keyvalues_sequence(&key_values);

    Ok((
        data,
        Package {
            id: package_id,
            checksum: SHA1::new(checksum.try_into().unwrap()),
            change_number,
            pics,
            key_values,
        },
    ))
}

pub fn parse_keyvalues(data: &[u8]) -> Result<KeyValues, VdfrError> {
    let (_, key_values) =
        parse_bytes_kv(data, &KeyValueOptions::default()).map_err(throw_nom_custom_error)?;
    let key_values = map_keyvalues_sequence(&key_values);
    Ok(key_values)
}

fn parse_bytes_kv<'a>(
    data: &'a [u8],
    options: &'a KeyValueOptions,
) -> IResult<&'a [u8], KeyValues, VdfrNomError> {
    let bin_end = if options.alt_format {
        BIN_END_ALT
    } else {
        BIN_END
    };

    let mut node = KeyValues::new();

    let mut data = data;
    loop {
        let (res, bin) = le_u8(data)?;

        if bin == bin_end {
            return Ok((res, node));
        }

        let (res, key) = if options.string_pool.is_empty() {
            parse_utf8(res)?
        } else {
            let (res, index) = le_u32(res)?;
            let index = index as usize;
            if index >= options.string_pool.len() {
                // use empty input
                let error_data =
                    VdfrNomError::from_error_kind(&[], nom::error::ErrorKind::LengthValue)
                        .with_message(
                            format!(
                                "index out of bounds in string pool (index: {}, pool size: {})",
                                index,
                                options.string_pool.len()
                            )
                            .as_str(),
                        );
                return Err(nom::Err::Failure(error_data));
            }
            (res, options.string_pool[index].clone())
        };

        let (res, value) = match bin {
            BIN_KV => {
                let (res, subnode) = parse_bytes_kv(res, options)?;
                (res, Value::KeyValueType(subnode))
            }
            BIN_STRING => {
                let (res, value) = parse_utf8(res)?;
                (res, Value::StringType(value))
            }
            BIN_WIDESTRING => {
                let (res, value) = parse_utf16(res)?;
                (res, Value::WideStringType(value))
            }
            BIN_INT32 | BIN_POINTER | BIN_COLOR => {
                let (res, value) = le_i32(res)?;
                let value = match bin {
                    BIN_INT32 => Value::Int32Type(value),
                    BIN_POINTER => Value::PointerType(value),
                    BIN_COLOR => Value::ColorType(value),
                    _ => unreachable!(),
                };
                (res, value)
            }
            BIN_UINT64 => {
                let (res, value) = le_u64(res)?;
                (res, Value::UInt64Type(value))
            }
            BIN_INT64 => {
                let (res, value) = le_i64(res)?;
                (res, Value::Int64Type(value))
            }
            BIN_FLOAT32 => {
                let (res, value) = le_f32(res)?;
                (res, Value::Float32Type(value))
            }
            _ => {
                let error_data =
                    VdfrNomError::from_error_kind(&[bin], nom::error::ErrorKind::LengthValue)
                        .with_message(
                            format!(
                                "unknown type in key-values (type: {}, key: {})",
                                bin,
                                key.as_str()
                            )
                            .as_str(),
                        );
                return Err(nom::Err::Failure(error_data));
            }
        };

        node.insert(key, value);
        data = res;
    }
}

fn read_string_pools(data: &[u8], amount: usize) -> IResult<&[u8], Vec<String>, VdfrNomError> {
    count(parse_utf8, amount)(data)
}

fn parse_utf8(input: &[u8]) -> IResult<&[u8], String, VdfrNomError> {
    // Parse until NULL byte
    let (rest, buf) = take_until("\0")(input)?;
    let (rest, _) = le_u8(rest)?; // Skip NULL byte
    let s = std::str::from_utf8(buf).map_err(|_| {
        nom::Err::Failure(
            VdfrNomError::from_error_kind(buf, nom::error::ErrorKind::Char)
                .with_message("Failed to parse UTF-8 string"),
        )
    })?;
    Ok((rest, s.to_string()))
}

enum Endian {
    Be,
    Le,
}

fn parse_utf16(input: &[u8]) -> IResult<&[u8], String, VdfrNomError> {
    // Parse until NULL byte
    let (rest, buf) = take_until("\0\0")(input)?;
    // Check if BOM is preset, if not assume BE
    let (buf, bom) = if buf.len() >= 2 {
        // Has BOM, check if LE or BE
        let big_endian = buf[0] == 0xFE && buf[1] == 0xFF;
        let little_endian = buf[0] == 0xFF && buf[1] == 0xFE;

        match (big_endian, little_endian) {
            // If BE/LE, skip BOM bytes and set endianness
            (true, false) => (&buf[2..], Endian::Be),
            (false, true) => (&buf[2..], Endian::Le),
            _ => (buf, Endian::Be),
        }
    } else {
        // No BOM, assume BE
        (buf, Endian::Be)
    };

    // Consume NULL byte
    let (rest, _) = match bom {
        Endian::Be => be_u16(rest)?,
        Endian::Le => le_u16(rest)?,
    };

    let mut v: Vec<u16> = vec![];
    for i in 0..buf.len() / 2 {
        let temp_buf = [buf[i * 2], buf[i * 2 + 1]];
        let c = match bom {
            Endian::Be => u16::from_be_bytes(temp_buf),
            Endian::Le => u16::from_le_bytes(temp_buf),
        };
        v.push(c);
    }
    v.push(0); // Add NULL terminator
    let s = String::from_utf16(&v).map_err(|_| {
        nom::Err::Failure(
            VdfrNomError::from_error_kind(buf, nom::error::ErrorKind::Char)
                .with_message("Failed to parse UTF-16 string"),
        )
    })?;
    Ok((rest, s))
}

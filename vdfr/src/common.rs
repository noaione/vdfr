use std::{collections::BTreeMap, ops::Deref};

#[cfg(feature = "serde")]
use serde::{ser::SerializeStruct, Deserialize, Serialize};

pub(crate) const BIN_KV: u8 = b'\x00';
pub(crate) const BIN_STRING: u8 = b'\x01';
pub(crate) const BIN_INT32: u8 = b'\x02';
pub(crate) const BIN_FLOAT32: u8 = b'\x03';
pub(crate) const BIN_POINTER: u8 = b'\x04';
pub(crate) const BIN_WIDESTRING: u8 = b'\x05';
pub(crate) const BIN_COLOR: u8 = b'\x06';
pub(crate) const BIN_UINT64: u8 = b'\x07';
pub(crate) const BIN_END: u8 = b'\x08';
pub(crate) const BIN_INT64: u8 = b'\x0A';
pub(crate) const BIN_END_ALT: u8 = b'\x0B';

pub(crate) const MAGIC_27: u32 = 0x07_56_44_27;
pub(crate) const MAGIC_28: u32 = 0x07_56_44_28;
pub(crate) const MAGIC_29: u32 = 0x07_56_44_29;

#[derive(Clone, Default)]
pub struct SHA1([u8; 20]);

impl SHA1 {
    pub fn new(data: [u8; 20]) -> Self {
        SHA1(data)
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl Deref for SHA1 {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "serde")]
impl Serialize for SHA1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:02x?}", self))
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SHA1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let mut data = [0; 20];
        for (i, c) in s.as_bytes().chunks(2).enumerate() {
            data[i] = u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16)
                .map_err(serde::de::Error::custom)?;
        }

        Ok(SHA1(data))
    }
}

impl std::fmt::Debug for SHA1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let merged = self
            .0
            .iter()
            .fold(String::new(), |acc, x| acc + &format!("{:02x}", x));
        write!(f, "{}", merged)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppInfoVersion {
    V27,
    V28,
    V29,
}

#[cfg(feature = "serde")]
impl Serialize for AppInfoVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32((*self).into())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for AppInfoVersion {
    fn deserialize<D>(deserializer: D) -> Result<AppInfoVersion, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v: u32 = Deserialize::deserialize(deserializer)?;
        v.try_into().map_err(serde::de::Error::custom)
    }
}

impl TryInto<AppInfoVersion> for u32 {
    type Error = VdfrError;

    fn try_into(self) -> Result<AppInfoVersion, VdfrError> {
        match self {
            MAGIC_27 => Ok(AppInfoVersion::V27),
            MAGIC_28 => Ok(AppInfoVersion::V28),
            MAGIC_29 => Ok(AppInfoVersion::V29),
            _ => Err(VdfrError::UnknownMagic(self)),
        }
    }
}

impl From<AppInfoVersion> for u32 {
    fn from(v: AppInfoVersion) -> u32 {
        match v {
            AppInfoVersion::V27 => MAGIC_27,
            AppInfoVersion::V28 => MAGIC_28,
            AppInfoVersion::V29 => MAGIC_29,
        }
    }
}

impl std::fmt::Display for AppInfoVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppInfoVersion::V27 => write!(f, "v27"),
            AppInfoVersion::V28 => write!(f, "v28"),
            AppInfoVersion::V29 => write!(f, "v29"),
        }
    }
}

#[derive(Debug)]
pub enum VdfrError {
    InvalidType(u8),
    ReadError(std::io::Error),
    UnknownMagic(u32),
    NomError(String),
    InvalidStringIndex(usize, usize),
}

impl std::error::Error for VdfrError {}

impl std::fmt::Display for VdfrError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VdfrError::InvalidType(t) => write!(f, "Invalid type {:#x}", t),
            VdfrError::UnknownMagic(v) => write!(f, "Unknown magic {:#x}", v),
            VdfrError::InvalidStringIndex(c, t) => {
                write!(f, "Invalid string index {} (total {})", c, t)
            }
            VdfrError::ReadError(e) => e.fmt(f),
            VdfrError::NomError(e) => write!(f, "Nom error: {}", e),
        }
    }
}

impl From<std::io::Error> for VdfrError {
    fn from(e: std::io::Error) -> Self {
        VdfrError::ReadError(e)
    }
}

#[derive(Clone)]
pub enum Value {
    StringType(String),
    WideStringType(String),
    Int32Type(i32),
    PointerType(i32),
    ColorType(i32),
    UInt64Type(u64),
    Int64Type(i64),
    Float32Type(f32),
    KeyValueType(KeyValues),
    ArrayType(Vec<Value>),
}

impl Value {
    pub(crate) fn save_bin<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            Value::StringType(_) => {
                writer.write_all(&[BIN_STRING])?;
            }
            Value::WideStringType(_) => {
                writer.write_all(&[BIN_WIDESTRING])?;
            }
            Value::Int32Type(_) => {
                writer.write_all(&[BIN_INT32])?;
            }
            Value::PointerType(_) => {
                writer.write_all(&[BIN_POINTER])?;
            }
            Value::ColorType(_) => {
                writer.write_all(&[BIN_COLOR])?;
            }
            Value::UInt64Type(_) => {
                writer.write_all(&[BIN_UINT64])?;
            }
            Value::Int64Type(_) => {
                writer.write_all(&[BIN_INT64])?;
            }
            Value::Float32Type(_) => {
                writer.write_all(&[BIN_FLOAT32])?;
            }
            Value::KeyValueType(_) => {
                writer.write_all(&[BIN_KV])?;
            }
            Value::ArrayType(_) => {
                // Array type is KeyValueType
                writer.write_all(&[BIN_KV])?;
            }
        }

        Ok(())
    }

    #[cfg(feature = "serde")]
    fn as_serde_json_value(&self) -> serde_json::Value {
        match self {
            Value::StringType(s) | Value::WideStringType(s) => serde_json::Value::String(s.clone()),
            Value::Int32Type(i) | Value::PointerType(i) | Value::ColorType(i) => {
                serde_json::Value::Number(serde_json::Number::from(*i))
            }
            Value::UInt64Type(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
            Value::Int64Type(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
            Value::Float32Type(i) => {
                serde_json::Value::Number(serde_json::Number::from_f64(f64::from(*i)).unwrap())
            }
            Value::KeyValueType(kv) => {
                let map: serde_json::Map<String, serde_json::Value> = kv
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_serde_json_value()))
                    .collect();
                serde_json::Value::Object(map)
            }
            Value::ArrayType(array) => {
                let veca = array.iter().map(|v| v.as_serde_json_value()).collect();
                serde_json::Value::Array(veca)
            }
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::StringType(s) | Value::WideStringType(s) => serializer.serialize_str(s),
            Value::Int32Type(i) | Value::PointerType(i) | Value::ColorType(i) => {
                serializer.serialize_i32(*i)
            }
            Value::UInt64Type(i) => serializer.serialize_u64(*i),
            Value::Int64Type(i) => serializer.serialize_i64(*i),
            Value::Float32Type(i) => serializer.serialize_f32(*i),
            Value::KeyValueType(kv) => kv.serialize(serializer),
            Value::ArrayType(array) => array.serialize(serializer),
        }
    }
}

fn fmt_string(s: &str) -> String {
    // escape quotes and backslashes
    let mut escaped = String::new();
    for c in s.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            _ => escaped.push(c),
        }
    }
    escaped
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::StringType(s) => write!(f, "\"{}\"", fmt_string(s)),
            Value::WideStringType(s) => write!(f, "W\"{}\"", fmt_string(s)),
            Value::Int32Type(i) => write!(f, "{}", i),
            Value::PointerType(i) => write!(f, "\"*{}\"", i),
            Value::ColorType(i) => write!(f, "{}", i),
            Value::UInt64Type(i) => write!(f, "{}", i),
            Value::Int64Type(i) => write!(f, "{}", i),
            Value::Float32Type(i) => write!(f, "{}", i),
            Value::KeyValueType(kv) => write!(f, "{:?}", kv),
            Value::ArrayType(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", v)?;
                }
                write!(f, "]")
            }
        }
    }
}

pub type KeyValues = BTreeMap<String, Value>;

/// Options for reading key-value data.
#[derive(Debug, Clone, Default)]
pub struct KeyValueOptions {
    pub string_pool: Vec<String>,
    pub alt_format: bool,
}

#[derive(Clone)]
pub struct App {
    pub id: u32,
    pub size: u32,
    pub state: u32,
    pub last_update: u32,
    pub access_token: u64,
    pub checksum_txt: SHA1,
    pub checksum_bin: Option<SHA1>,
    pub change_number: u32,
    pub key_values: KeyValues,
}

#[cfg(feature = "serde")]
impl serde::Serialize for App {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("App", 9)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("size", &self.size)?;
        state.serialize_field("state", &self.state)?;
        state.serialize_field("last_update", &self.last_update)?;
        state.serialize_field("access_token", &self.access_token)?;
        state.serialize_field("checksum_txt", &self.checksum_sha1_txt())?;
        state.serialize_field("checksum_bin", &self.checksum_sha1_bin())?;
        state.serialize_field("change_number", &self.change_number)?;
        state.serialize_field("key_values", &self.key_values)?;
        state.end()
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("id", &self.id)
            .field("size", &self.size)
            .field("state", &self.state)
            .field("last_update", &self.last_update)
            .field("access_token", &self.access_token)
            .field("checksum_txt", &self.checksum_sha1_txt())
            .field("checksum_bin", &self.checksum_sha1_bin())
            .field("change_number", &self.change_number)
            .field("key_values", &self.key_values)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub version: AppInfoVersion,
    pub universe: u32,
    pub apps: BTreeMap<u32, App>,
}

#[cfg(feature = "serde")]
impl serde::Serialize for AppInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("AppInfo", 3)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("universe", &self.universe)?;
        state.serialize_field("apps", &self.apps)?;
        state.end()
    }
}

impl App {
    pub fn get(&self, keys: &[&str]) -> Option<&Value> {
        find_keys(&self.key_values, keys)
    }

    pub fn checksum_sha1_txt(&self) -> String {
        format!("{:02x?}", self.checksum_txt)
    }

    pub fn checksum_sha1_bin(&self) -> Option<String> {
        self.checksum_bin
            .as_ref()
            .map(|sha1| format!("{:02x?}", sha1))
    }

    /// Convert the key-values to a serde JSON object.
    #[cfg(feature = "serde")]
    pub fn as_serde_keyvalues(&self) -> serde_json::Value {
        let map: serde_json::Map<String, serde_json::Value> = self
            .key_values
            .iter()
            .map(|(k, v)| (k.clone(), v.as_serde_json_value()))
            .collect();
        serde_json::Value::Object(map)
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub id: u32,
    pub checksum: SHA1,
    pub change_number: u32,
    pub pics: u64,
    pub key_values: KeyValues,
}

#[cfg(feature = "serde")]
impl serde::Serialize for Package {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Package", 5)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("checksum", &self.checksum)?;
        state.serialize_field("change_number", &self.change_number)?;
        state.serialize_field("pics", &self.pics)?;
        state.serialize_field("key_values", &self.key_values)?;
        state.end()
    }
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub version: u32,
    pub universe: u32,
    pub packages: BTreeMap<u32, Package>,
}

#[cfg(feature = "serde")]
impl serde::Serialize for PackageInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PackageInfo", 3)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("universe", &self.universe)?;
        state.serialize_field("packages", &self.packages)?;
        state.end()
    }
}

impl Package {
    pub fn get(&self, keys: &[&str]) -> Option<&Value> {
        find_keys(&self.key_values, keys)
    }
}

/// Map a KeyValueType to a sequence of key-values
/// If the mapping is "0" -> "Item", "1" -> "Item", etc.
///
/// If not, keep the original key-value mapping
pub(crate) fn map_keyvalues_sequence(key_values: &KeyValues) -> KeyValues {
    key_values
        .iter()
        .map(|(key, value)| {
            let data = map_value_data(value);
            (key.clone(), data)
        })
        .collect()
}

fn map_value_data(value: &Value) -> Value {
    // This doesn't have ArrayType at all
    match value {
        Value::KeyValueType(sub_kv) => {
            let total_keys = sub_kv.len();
            let mut keys = sub_kv
                .keys()
                .filter_map(|k| k.parse::<usize>().ok())
                .collect::<Vec<usize>>();
            keys.sort();

            // Check if keys is a sequence of numbers
            let is_sequence = if keys.is_empty() || total_keys != keys.len() {
                // If empty, it's not a sequence
                false
            } else {
                keys.iter().enumerate().all(|(i, &key)| i == key)
            };

            if is_sequence {
                let kv_array: Vec<Value> = keys
                    .iter()
                    // Map and collect the actual values data
                    .map(|&key| map_value_data(sub_kv.get(&key.to_string()).unwrap()))
                    .collect();
                // Return as an array
                Value::ArrayType(kv_array)
            } else {
                // If not sequence, call recursively
                Value::KeyValueType(map_keyvalues_sequence(sub_kv))
            }
        }
        _ => value.clone(),
    }
}

// Recursively search for the specified sequence of keys in the key-value data.
// The order of the keys dictates the hierarchy, with all except the last having
// to be a Value::KeyValueType.
fn find_keys<'a>(kv: &'a KeyValues, keys: &[&str]) -> Option<&'a Value> {
    if keys.is_empty() {
        return None;
    }

    let key = *keys.first().unwrap();
    let value = kv.get(&key.to_string());
    if keys.len() == 1 {
        value
    } else {
        find_key_next(value, &keys[1..])
    }
}

fn find_key_next<'a>(value: Option<&'a Value>, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Some(Value::KeyValueType(kv)) => find_keys(kv, keys),
        Some(Value::ArrayType(array)) => {
            // Check next key is a number
            if let Ok(index) = keys.first().unwrap().parse::<usize>() {
                let value = array.get(index);

                // If the value is a KeyValueType, call recursively
                // If not, return None
                find_key_next(value, &keys[1..])
            } else {
                None
            }
        }
        _ => None,
    }
}

//! Binary schema loader: loads pre-compiled sproto binary schemas.
//!
//! The binary schema format is a self-describing sproto message with this structure:
//! ```text
//! .type {
//!     .field {
//!         name 0 : string
//!         buildin 1 : integer
//!         type 2 : integer
//!         tag 3 : integer
//!         array 4 : boolean
//!         key 5 : integer
//!         map 6 : boolean
//!     }
//!     name 0 : string
//!     fields 1 : *field
//! }
//! .protocol {
//!     name 0 : string
//!     tag 1 : integer
//!     request 2 : integer   # type index
//!     response 3 : integer  # type index
//!     confirm 4 : boolean
//! }
//! .group {
//!     type 0 : *type
//!     protocol 1 : *protocol
//! }
//! ```

use std::collections::HashMap;

use crate::codec::wire::*;
use crate::error::DecodeError;
use crate::types::*;

/// Load a pre-compiled binary schema into a `Sproto` object.
///
/// This is equivalent to the C function `sproto_create()`.
pub fn load_binary(data: &[u8]) -> Result<Sproto, DecodeError> {
    let sz = data.len();
    if sz < SIZEOF_HEADER {
        return Err(DecodeError::Truncated { need: SIZEOF_HEADER, have: sz });
    }

    let fn_count = read_u16_le(data) as usize;
    if fn_count == 0 || fn_count > 2 {
        return Err(DecodeError::InvalidData("group must have 1 or 2 fields".into()));
    }

    let field_part_end = SIZEOF_HEADER + fn_count * SIZEOF_FIELD;
    if sz < field_part_end {
        return Err(DecodeError::Truncated { need: field_part_end, have: sz });
    }

    // All fields in group must be 0 (data in data part)
    for i in 0..fn_count {
        let v = read_u16_le(&data[SIZEOF_HEADER + i * SIZEOF_FIELD..]);
        if v != 0 {
            return Err(DecodeError::InvalidData("group fields must be in data part".into()));
        }
    }

    let mut content_offset = field_part_end;

    // Read type array
    let raw_types: Vec<RawType>;
    {
        if content_offset + SIZEOF_LENGTH > sz {
            return Err(DecodeError::Truncated { need: content_offset + SIZEOF_LENGTH, have: sz });
        }
        let arr_sz = read_u32_le(&data[content_offset..]) as usize;
        let arr_data = &data[content_offset + SIZEOF_LENGTH..content_offset + SIZEOF_LENGTH + arr_sz];
        raw_types = decode_type_array(arr_data)?;
        content_offset += SIZEOF_LENGTH + arr_sz;
    }

    // Read protocol array (optional)
    let mut raw_protocols: Vec<RawProtocol> = Vec::new();
    if fn_count == 2 {
        if content_offset + SIZEOF_LENGTH > sz {
            return Err(DecodeError::Truncated { need: content_offset + SIZEOF_LENGTH, have: sz });
        }
        let arr_sz = read_u32_le(&data[content_offset..]) as usize;
        let arr_data = &data[content_offset + SIZEOF_LENGTH..content_offset + SIZEOF_LENGTH + arr_sz];
        raw_protocols = decode_protocol_array(arr_data)?;
    }

    // Build Sproto from raw data
    build_sproto(raw_types, raw_protocols)
}

// --- Internal types ---

struct RawType {
    name: String,
    fields: Vec<RawField>,
}

struct RawField {
    name: String,
    builtin: Option<u16>,  // 0=integer, 1=boolean, 2=string, 3=double
    type_index: Option<u16>,
    tag: u16,
    array: bool,
    key: Option<u16>,
    map: bool,
}

struct RawProtocol {
    name: String,
    tag: u16,
    request: Option<u16>,
    response: Option<u16>,
    confirm: bool,
}

// --- Decode helpers ---

/// Decode a struct from binary data, returning field values as (tag -> value) pairs.
/// Values with tag in data part are returned as (tag, None, data_slice).
/// Inline values are returned as (tag, Some(value), &[]).
type DecodedFields<'a> = Vec<(u16, Option<i32>, &'a [u8])>;

fn decode_struct_fields(data: &[u8]) -> Result<DecodedFields<'_>, DecodeError> {
    let sz = data.len();
    if sz < SIZEOF_HEADER {
        return Err(DecodeError::Truncated { need: SIZEOF_HEADER, have: sz });
    }

    let fn_count = read_u16_le(data) as usize;
    let field_end = SIZEOF_HEADER + fn_count * SIZEOF_FIELD;
    if sz < field_end {
        return Err(DecodeError::Truncated { need: field_end, have: sz });
    }

    let mut results = Vec::new();
    let mut data_offset = field_end;
    let mut tag: i32 = -1;

    for i in 0..fn_count {
        let value = read_u16_le(&data[SIZEOF_HEADER + i * SIZEOF_FIELD..]) as i32;
        tag += 1;

        if value & 1 != 0 {
            tag += value / 2;
            continue;
        }

        let decoded = value / 2 - 1;

        if decoded < 0 {
            // Data in data part
            if data_offset + SIZEOF_LENGTH > sz {
                return Err(DecodeError::Truncated { need: data_offset + SIZEOF_LENGTH, have: sz });
            }
            let dsz = read_u32_le(&data[data_offset..]) as usize;
            if data_offset + SIZEOF_LENGTH + dsz > sz {
                return Err(DecodeError::Truncated { need: data_offset + SIZEOF_LENGTH + dsz, have: sz });
            }
            let field_data = &data[data_offset + SIZEOF_LENGTH..data_offset + SIZEOF_LENGTH + dsz];
            results.push((tag as u16, None, field_data));
            data_offset += SIZEOF_LENGTH + dsz;
        } else {
            results.push((tag as u16, Some(decoded), &[]));
        }
    }

    Ok(results)
}

fn decode_string(data: &[u8]) -> Result<String, DecodeError> {
    String::from_utf8(data.to_vec()).map_err(|e| DecodeError::InvalidData(format!("invalid UTF-8: {}", e)))
}

fn decode_type_array(data: &[u8]) -> Result<Vec<RawType>, DecodeError> {
    let mut types = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        if offset + SIZEOF_LENGTH > data.len() {
            return Err(DecodeError::Truncated { need: offset + SIZEOF_LENGTH, have: data.len() });
        }
        let elem_sz = read_u32_le(&data[offset..]) as usize;
        let elem_data = &data[offset + SIZEOF_LENGTH..offset + SIZEOF_LENGTH + elem_sz];
        types.push(decode_single_type(elem_data)?);
        offset += SIZEOF_LENGTH + elem_sz;
    }

    Ok(types)
}

fn decode_single_type(data: &[u8]) -> Result<RawType, DecodeError> {
    let fields_decoded = decode_struct_fields(data)?;

    let mut name = String::new();
    let mut raw_fields: Vec<RawField> = Vec::new();

    for (tag, _inline_val, field_data) in &fields_decoded {
        match tag {
            0 => {
                // name (always in data part)
                name = decode_string(field_data)?;
            }
            1 => {
                // fields array (in data part)
                raw_fields = decode_field_array(field_data)?;
            }
            _ => {} // ignore unknown
        }
    }

    Ok(RawType { name, fields: raw_fields })
}

fn decode_field_array(data: &[u8]) -> Result<Vec<RawField>, DecodeError> {
    let mut fields = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        if offset + SIZEOF_LENGTH > data.len() {
            return Err(DecodeError::Truncated { need: offset + SIZEOF_LENGTH, have: data.len() });
        }
        let elem_sz = read_u32_le(&data[offset..]) as usize;
        let elem_data = &data[offset + SIZEOF_LENGTH..offset + SIZEOF_LENGTH + elem_sz];
        fields.push(decode_single_field(elem_data)?);
        offset += SIZEOF_LENGTH + elem_sz;
    }

    Ok(fields)
}

fn decode_single_field(data: &[u8]) -> Result<RawField, DecodeError> {
    let entries = decode_struct_fields(data)?;

    let mut name = String::new();
    let mut builtin: Option<u16> = None;
    let mut type_index: Option<u16> = None;
    let mut tag: u16 = 0;
    let mut array = false;
    let mut key: Option<u16> = None;
    let mut map = false;

    for (ftag, inline_val, field_data) in &entries {
        match ftag {
            0 => {
                // name
                name = decode_string(field_data)?;
            }
            1 => {
                // buildin
                builtin = inline_val.map(|v| v as u16);
            }
            2 => {
                // type (index or precision or binary flag)
                type_index = inline_val.map(|v| v as u16);
            }
            3 => {
                // tag
                tag = inline_val.unwrap_or(0) as u16;
            }
            4 => {
                // array
                array = inline_val.map(|v| v != 0).unwrap_or(false);
            }
            5 => {
                // key
                key = inline_val.map(|v| v as u16);
            }
            6 => {
                // map
                map = inline_val.map(|v| v != 0).unwrap_or(false);
            }
            _ => {} // ignore unknown
        }
    }

    Ok(RawField {
        name,
        builtin,
        type_index,
        tag,
        array,
        key,
        map,
    })
}

fn decode_protocol_array(data: &[u8]) -> Result<Vec<RawProtocol>, DecodeError> {
    let mut protocols = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        if offset + SIZEOF_LENGTH > data.len() {
            return Err(DecodeError::Truncated { need: offset + SIZEOF_LENGTH, have: data.len() });
        }
        let elem_sz = read_u32_le(&data[offset..]) as usize;
        let elem_data = &data[offset + SIZEOF_LENGTH..offset + SIZEOF_LENGTH + elem_sz];
        protocols.push(decode_single_protocol(elem_data)?);
        offset += SIZEOF_LENGTH + elem_sz;
    }

    Ok(protocols)
}

fn decode_single_protocol(data: &[u8]) -> Result<RawProtocol, DecodeError> {
    let entries = decode_struct_fields(data)?;

    let mut name = String::new();
    let mut tag: u16 = 0;
    let mut request: Option<u16> = None;
    let mut response: Option<u16> = None;
    let mut confirm = false;

    for (ftag, inline_val, field_data) in &entries {
        match ftag {
            0 => {
                // name
                name = decode_string(field_data)?;
            }
            1 => {
                // tag
                tag = inline_val.unwrap_or(0) as u16;
            }
            2 => {
                // request type index
                request = inline_val.map(|v| v as u16);
            }
            3 => {
                // response type index
                response = inline_val.map(|v| v as u16);
            }
            4 => {
                // confirm
                confirm = inline_val.map(|v| v != 0).unwrap_or(false);
            }
            _ => {}
        }
    }

    Ok(RawProtocol { name, tag, request, response, confirm })
}

// --- Build Sproto ---

fn calc_pow(base: u32, n: u16) -> u32 {
    base.pow(n as u32)
}

fn build_sproto(raw_types: Vec<RawType>, raw_protocols: Vec<RawProtocol>) -> Result<Sproto, DecodeError> {
    let type_n = raw_types.len();

    // Build types
    let mut types_list: Vec<SprotoType> = Vec::with_capacity(type_n);
    let mut types_by_name: HashMap<String, usize> = HashMap::new();

    for (idx, rt) in raw_types.iter().enumerate() {
        let mut fields: Vec<Field> = Vec::new();

        for rf in &rt.fields {
            let field_type = if let Some(b) = rf.builtin {
                match b {
                    0 => FieldType::Integer,
                    1 => FieldType::Boolean,
                    2 => {
                        if rf.type_index == Some(1) {
                            FieldType::Binary
                        } else {
                            FieldType::String
                        }
                    }
                    3 => FieldType::Double,
                    _ => return Err(DecodeError::InvalidData(format!("invalid builtin type {}", b))),
                }
            } else if let Some(ti) = rf.type_index {
                if (ti as usize) >= type_n {
                    return Err(DecodeError::InvalidData(format!("type index {} out of range", ti)));
                }
                FieldType::Struct(ti as usize)
            } else {
                return Err(DecodeError::InvalidData(format!("field '{}' has no type", rf.name)));
            };

            let decimal_precision = if rf.builtin == Some(0) {
                // Integer type: type_index is the precision exponent
                rf.type_index.map(|p| calc_pow(10, p)).unwrap_or(0)
            } else {
                0
            };

            fields.push(Field {
                name: rf.name.clone(),
                tag: rf.tag,
                field_type,
                is_array: rf.array,
                key_tag: rf.key.map(|k| k as i32).unwrap_or(-1),
                is_map: rf.map,
                decimal_precision,
            });
        }

        // Fields should already be sorted by tag from the binary format
        fields.sort_by_key(|f| f.tag);

        let (base_tag, maxn) = compute_base_tag_and_maxn(&fields);

        types_by_name.insert(rt.name.clone(), idx);
        types_list.push(SprotoType {
            name: rt.name.clone(),
            fields,
            base_tag,
            maxn,
        });
    }

    // Build protocols
    let mut protocols: Vec<Protocol> = Vec::new();
    let mut protocols_by_name: HashMap<String, usize> = HashMap::new();
    let mut protocols_by_tag: HashMap<u16, usize> = HashMap::new();

    for rp in raw_protocols {
        let idx = protocols.len();
        protocols_by_name.insert(rp.name.clone(), idx);
        protocols_by_tag.insert(rp.tag, idx);

        let request = rp.request.map(|r| {
            if (r as usize) >= type_n {
                Err(DecodeError::InvalidData(format!("request type index {} out of range", r)))
            } else {
                Ok(r as usize)
            }
        }).transpose()?;

        let response = rp.response.map(|r| {
            if (r as usize) >= type_n {
                Err(DecodeError::InvalidData(format!("response type index {} out of range", r)))
            } else {
                Ok(r as usize)
            }
        }).transpose()?;

        protocols.push(Protocol {
            name: rp.name,
            tag: rp.tag,
            request,
            response,
            confirm: rp.confirm,
        });
    }

    Ok(Sproto {
        types_list,
        types_by_name,
        protocols,
        protocols_by_name,
        protocols_by_tag,
    })
}

fn compute_base_tag_and_maxn(fields: &[Field]) -> (i32, usize) {
    if fields.is_empty() {
        return (-1, 0);
    }

    let n = fields.len();
    let mut maxn = n;
    let mut last: i32 = -1;

    for f in fields {
        let tag = f.tag as i32;
        if tag > last + 1 {
            maxn += 1;
        }
        last = tag;
    }

    let base = fields[0].tag as i32;
    let span = fields[n - 1].tag as i32 - base + 1;
    let base_tag = if span as usize != n { -1 } else { base };

    (base_tag, maxn)
}

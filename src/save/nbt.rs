use std::fmt;
use std::io::{self, Cursor, Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TagType {
    End = 0,
    Byte = 1,
    Short = 2,
    Int = 3,
    Long = 4,
    Float = 5,
    Double = 6,
    ByteArray = 7,
    String = 8,
    List = 9,
    Compound = 10,
    IntArray = 11,
}

impl TryFrom<u8> for TagType {
    type Error = NbtError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::End),
            1 => Ok(Self::Byte),
            2 => Ok(Self::Short),
            3 => Ok(Self::Int),
            4 => Ok(Self::Long),
            5 => Ok(Self::Float),
            6 => Ok(Self::Double),
            7 => Ok(Self::ByteArray),
            8 => Ok(Self::String),
            9 => Ok(Self::List),
            10 => Ok(Self::Compound),
            11 => Ok(Self::IntArray),
            _ => Err(NbtError::UnknownTagType(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NbtTag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(NbtList),
    Compound(NbtCompound),
    IntArray(Vec<i32>),
}

impl NbtTag {
    pub fn tag_type(&self) -> TagType {
        match self {
            Self::End => TagType::End,
            Self::Byte(_) => TagType::Byte,
            Self::Short(_) => TagType::Short,
            Self::Int(_) => TagType::Int,
            Self::Long(_) => TagType::Long,
            Self::Float(_) => TagType::Float,
            Self::Double(_) => TagType::Double,
            Self::ByteArray(_) => TagType::ByteArray,
            Self::String(_) => TagType::String,
            Self::List(_) => TagType::List,
            Self::Compound(_) => TagType::Compound,
            Self::IntArray(_) => TagType::IntArray,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NbtList {
    pub element_type: TagType,
    pub elements: Vec<NbtTag>,
}

impl NbtList {
    pub fn empty() -> Self {
        Self {
            element_type: TagType::Byte,
            elements: Vec::new(),
        }
    }

    pub fn from_elements(elements: Vec<NbtTag>) -> Result<Self, NbtError> {
        if elements.is_empty() {
            return Ok(Self::empty());
        }

        let expected = elements[0].tag_type();
        if expected == TagType::End {
            return Err(NbtError::InvalidListElementType(expected));
        }

        for element in &elements {
            let found = element.tag_type();
            if found != expected {
                return Err(NbtError::ListTypeMismatch { expected, found });
            }
        }

        Ok(Self {
            element_type: expected,
            elements,
        })
    }

    pub fn push(&mut self, tag: NbtTag) -> Result<(), NbtError> {
        let found = tag.tag_type();
        if found == TagType::End {
            return Err(NbtError::InvalidListElementType(found));
        }

        if self.elements.is_empty() {
            self.element_type = found;
        } else if self.element_type != found {
            return Err(NbtError::ListTypeMismatch {
                expected: self.element_type,
                found,
            });
        }

        self.elements.push(tag);
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NbtCompound {
    entries: Vec<(String, NbtTag)>,
}

impl NbtCompound {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &[(String, NbtTag)] {
        &self.entries
    }

    pub fn get(&self, name: &str) -> Option<&NbtTag> {
        self.entries
            .iter()
            .find(|(entry_name, _)| entry_name == name)
            .map(|(_, tag)| tag)
    }

    pub fn insert(&mut self, name: impl Into<String>, tag: NbtTag) -> Option<NbtTag> {
        let name = name.into();

        if let Some((_, existing_tag)) = self
            .entries
            .iter_mut()
            .find(|(existing_name, _)| *existing_name == name)
        {
            return Some(std::mem::replace(existing_tag, tag));
        }

        self.entries.push((name, tag));
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NbtRoot {
    pub name: String,
    pub compound: NbtCompound,
}

impl NbtRoot {
    pub fn new(name: impl Into<String>, compound: NbtCompound) -> Self {
        Self {
            name: name.into(),
            compound,
        }
    }
}

#[derive(Debug)]
pub enum NbtError {
    Io(io::Error),
    UnknownTagType(u8),
    InvalidRootTag(TagType),
    NegativeLength {
        tag: TagType,
        length: i32,
    },
    InvalidListElementType(TagType),
    ListTypeMismatch {
        expected: TagType,
        found: TagType,
    },
    UnexpectedEndTag,
    UtfTooLong(usize),
    LengthTooLarge {
        context: &'static str,
        length: usize,
    },
    InvalidUtfEncoding(String),
}

impl fmt::Display for NbtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::UnknownTagType(tag) => write!(f, "unknown nbt tag type: {tag}"),
            Self::InvalidRootTag(tag) => {
                write!(f, "invalid root tag type: expected Compound, found {tag:?}")
            }
            Self::NegativeLength { tag, length } => {
                write!(f, "negative length {length} for tag type {tag:?}")
            }
            Self::InvalidListElementType(tag) => {
                write!(f, "invalid list element type: {tag:?}")
            }
            Self::ListTypeMismatch { expected, found } => {
                write!(
                    f,
                    "list type mismatch: expected {expected:?}, found {found:?}"
                )
            }
            Self::UnexpectedEndTag => write!(f, "unexpected TAG_End payload"),
            Self::UtfTooLong(length) => write!(f, "modified utf payload too long: {length} bytes"),
            Self::LengthTooLarge { context, length } => {
                write!(f, "{context} length exceeds i32::MAX: {length}")
            }
            Self::InvalidUtfEncoding(reason) => write!(f, "invalid modified utf data: {reason}"),
        }
    }
}

impl std::error::Error for NbtError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for NbtError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn read_root_from_bytes(bytes: &[u8]) -> Result<NbtRoot, NbtError> {
    let mut cursor = Cursor::new(bytes);
    read_root(&mut cursor)
}

pub fn write_root_to_bytes(root: &NbtRoot) -> Result<Vec<u8>, NbtError> {
    let mut bytes = Vec::new();
    write_root(&mut bytes, root)?;
    Ok(bytes)
}

pub fn read_root<R: Read>(reader: &mut R) -> Result<NbtRoot, NbtError> {
    let root_type = read_tag_type(reader)?;
    if root_type != TagType::Compound {
        return Err(NbtError::InvalidRootTag(root_type));
    }

    let root_name = read_modified_utf(reader)?;
    let compound = read_compound_payload(reader)?;

    Ok(NbtRoot::new(root_name, compound))
}

pub fn write_root<W: Write>(writer: &mut W, root: &NbtRoot) -> Result<(), NbtError> {
    write_u8(writer, TagType::Compound as u8)?;
    write_modified_utf(writer, &root.name)?;
    write_compound_payload(writer, &root.compound)
}

fn read_compound_payload<R: Read>(reader: &mut R) -> Result<NbtCompound, NbtError> {
    let mut compound = NbtCompound::new();

    loop {
        let tag_type = read_tag_type(reader)?;
        if tag_type == TagType::End {
            break;
        }

        let name = read_modified_utf(reader)?;
        let payload = read_tag_payload(reader, tag_type)?;
        compound.insert(name, payload);
    }

    Ok(compound)
}

fn write_compound_payload<W: Write>(
    writer: &mut W,
    compound: &NbtCompound,
) -> Result<(), NbtError> {
    for (name, tag) in compound.entries() {
        write_named_tag(writer, name, tag)?;
    }

    write_u8(writer, TagType::End as u8)
}

fn read_tag_payload<R: Read>(reader: &mut R, tag_type: TagType) -> Result<NbtTag, NbtError> {
    match tag_type {
        TagType::End => Ok(NbtTag::End),
        TagType::Byte => Ok(NbtTag::Byte(read_i8(reader)?)),
        TagType::Short => Ok(NbtTag::Short(read_i16_be(reader)?)),
        TagType::Int => Ok(NbtTag::Int(read_i32_be(reader)?)),
        TagType::Long => Ok(NbtTag::Long(read_i64_be(reader)?)),
        TagType::Float => Ok(NbtTag::Float(f32::from_bits(read_u32_be(reader)?))),
        TagType::Double => Ok(NbtTag::Double(f64::from_bits(read_u64_be(reader)?))),
        TagType::ByteArray => {
            let length = read_length(reader, TagType::ByteArray)?;
            let mut bytes = vec![0_u8; length];
            reader.read_exact(&mut bytes)?;

            let signed_bytes = bytes
                .into_iter()
                .map(|byte| i8::from_be_bytes([byte]))
                .collect();
            Ok(NbtTag::ByteArray(signed_bytes))
        }
        TagType::String => Ok(NbtTag::String(read_modified_utf(reader)?)),
        TagType::List => {
            let element_type = read_tag_type(reader)?;
            let length = read_length(reader, TagType::List)?;

            if length > 0 && element_type == TagType::End {
                return Err(NbtError::InvalidListElementType(element_type));
            }

            let mut elements = Vec::with_capacity(length);
            for _ in 0..length {
                elements.push(read_tag_payload(reader, element_type)?);
            }

            Ok(NbtTag::List(NbtList {
                element_type,
                elements,
            }))
        }
        TagType::Compound => Ok(NbtTag::Compound(read_compound_payload(reader)?)),
        TagType::IntArray => {
            let length = read_length(reader, TagType::IntArray)?;
            let mut values = Vec::with_capacity(length);

            for _ in 0..length {
                values.push(read_i32_be(reader)?);
            }

            Ok(NbtTag::IntArray(values))
        }
    }
}

fn write_named_tag<W: Write>(writer: &mut W, name: &str, tag: &NbtTag) -> Result<(), NbtError> {
    let tag_type = tag.tag_type();
    if tag_type == TagType::End {
        return Err(NbtError::UnexpectedEndTag);
    }

    write_u8(writer, tag_type as u8)?;
    write_modified_utf(writer, name)?;
    write_tag_payload(writer, tag)
}

fn write_tag_payload<W: Write>(writer: &mut W, tag: &NbtTag) -> Result<(), NbtError> {
    match tag {
        NbtTag::End => Ok(()),
        NbtTag::Byte(value) => write_i8(writer, *value),
        NbtTag::Short(value) => write_i16_be(writer, *value),
        NbtTag::Int(value) => write_i32_be(writer, *value),
        NbtTag::Long(value) => write_i64_be(writer, *value),
        NbtTag::Float(value) => write_u32_be(writer, value.to_bits()),
        NbtTag::Double(value) => write_u64_be(writer, value.to_bits()),
        NbtTag::ByteArray(values) => {
            write_i32_be(writer, len_to_i32(values.len(), "byte array")?)?;

            let bytes: Vec<u8> = values.iter().map(|value| value.to_be_bytes()[0]).collect();
            writer.write_all(&bytes)?;

            Ok(())
        }
        NbtTag::String(value) => write_modified_utf(writer, value),
        NbtTag::List(list) => write_list_payload(writer, list),
        NbtTag::Compound(compound) => write_compound_payload(writer, compound),
        NbtTag::IntArray(values) => {
            write_i32_be(writer, len_to_i32(values.len(), "int array")?)?;
            for value in values {
                write_i32_be(writer, *value)?;
            }

            Ok(())
        }
    }
}

fn write_list_payload<W: Write>(writer: &mut W, list: &NbtList) -> Result<(), NbtError> {
    if list.elements.is_empty() {
        write_u8(writer, TagType::Byte as u8)?;
        return write_i32_be(writer, 0);
    }

    let element_type = list.elements[0].tag_type();
    if element_type == TagType::End {
        return Err(NbtError::InvalidListElementType(element_type));
    }

    for element in &list.elements {
        let found = element.tag_type();
        if found != element_type {
            return Err(NbtError::ListTypeMismatch {
                expected: element_type,
                found,
            });
        }
    }

    write_u8(writer, element_type as u8)?;
    write_i32_be(writer, len_to_i32(list.elements.len(), "list")?)?;

    for element in &list.elements {
        write_tag_payload(writer, element)?;
    }

    Ok(())
}

fn read_modified_utf<R: Read>(reader: &mut R) -> Result<String, NbtError> {
    let length = usize::from(read_u16_be(reader)?);
    let mut bytes = vec![0_u8; length];
    reader.read_exact(&mut bytes)?;
    decode_modified_utf8(&bytes)
}

fn write_modified_utf<W: Write>(writer: &mut W, value: &str) -> Result<(), NbtError> {
    let encoded = encode_modified_utf8(value)?;
    let length = u16::try_from(encoded.len()).map_err(|_| NbtError::UtfTooLong(encoded.len()))?;

    write_u16_be(writer, length)?;
    writer.write_all(&encoded)?;

    Ok(())
}

fn encode_modified_utf8(value: &str) -> Result<Vec<u8>, NbtError> {
    let mut encoded = Vec::new();

    for code_unit in value.encode_utf16() {
        let code = u32::from(code_unit);

        if (0x0001..=0x007F).contains(&code) {
            encoded.push(code as u8);
        } else if code > 0x07FF {
            encoded.push((0xE0 | ((code >> 12) & 0x0F)) as u8);
            encoded.push((0x80 | ((code >> 6) & 0x3F)) as u8);
            encoded.push((0x80 | (code & 0x3F)) as u8);
        } else {
            encoded.push((0xC0 | ((code >> 6) & 0x1F)) as u8);
            encoded.push((0x80 | (code & 0x3F)) as u8);
        }
    }

    if encoded.len() > usize::from(u16::MAX) {
        return Err(NbtError::UtfTooLong(encoded.len()));
    }

    Ok(encoded)
}

fn decode_modified_utf8(bytes: &[u8]) -> Result<String, NbtError> {
    let mut units = Vec::new();
    let mut index = 0_usize;

    while index < bytes.len() {
        let first = bytes[index];

        if (first & 0x80) == 0 {
            units.push(u16::from(first));
            index += 1;
            continue;
        }

        if (first & 0xE0) == 0xC0 {
            if index + 1 >= bytes.len() {
                return Err(NbtError::InvalidUtfEncoding(
                    "missing second byte in 2-byte sequence".to_string(),
                ));
            }

            let second = bytes[index + 1];
            if (second & 0xC0) != 0x80 {
                return Err(NbtError::InvalidUtfEncoding(
                    "invalid continuation byte in 2-byte sequence".to_string(),
                ));
            }

            let code = (u16::from(first & 0x1F) << 6) | u16::from(second & 0x3F);
            units.push(code);
            index += 2;
            continue;
        }

        if (first & 0xF0) == 0xE0 {
            if index + 2 >= bytes.len() {
                return Err(NbtError::InvalidUtfEncoding(
                    "incomplete 3-byte sequence".to_string(),
                ));
            }

            let second = bytes[index + 1];
            let third = bytes[index + 2];

            if (second & 0xC0) != 0x80 || (third & 0xC0) != 0x80 {
                return Err(NbtError::InvalidUtfEncoding(
                    "invalid continuation byte in 3-byte sequence".to_string(),
                ));
            }

            let code = (u16::from(first & 0x0F) << 12)
                | (u16::from(second & 0x3F) << 6)
                | u16::from(third & 0x3F);
            units.push(code);
            index += 3;
            continue;
        }

        return Err(NbtError::InvalidUtfEncoding(
            "unsupported leading byte pattern".to_string(),
        ));
    }

    String::from_utf16(&units)
        .map_err(|error| NbtError::InvalidUtfEncoding(format!("invalid utf16 data: {error}")))
}

fn read_tag_type<R: Read>(reader: &mut R) -> Result<TagType, NbtError> {
    let raw = read_u8(reader)?;
    TagType::try_from(raw)
}

fn read_length<R: Read>(reader: &mut R, tag: TagType) -> Result<usize, NbtError> {
    let length = read_i32_be(reader)?;
    if length < 0 {
        return Err(NbtError::NegativeLength { tag, length });
    }

    usize::try_from(length).map_err(|_| NbtError::NegativeLength { tag, length })
}

fn read_u8<R: Read>(reader: &mut R) -> Result<u8, NbtError> {
    let mut buffer = [0_u8; 1];
    reader.read_exact(&mut buffer)?;
    Ok(buffer[0])
}

fn read_i8<R: Read>(reader: &mut R) -> Result<i8, NbtError> {
    Ok(i8::from_be_bytes([read_u8(reader)?]))
}

fn read_u16_be<R: Read>(reader: &mut R) -> Result<u16, NbtError> {
    let mut buffer = [0_u8; 2];
    reader.read_exact(&mut buffer)?;
    Ok(u16::from_be_bytes(buffer))
}

fn read_i16_be<R: Read>(reader: &mut R) -> Result<i16, NbtError> {
    let mut buffer = [0_u8; 2];
    reader.read_exact(&mut buffer)?;
    Ok(i16::from_be_bytes(buffer))
}

fn read_i32_be<R: Read>(reader: &mut R) -> Result<i32, NbtError> {
    let mut buffer = [0_u8; 4];
    reader.read_exact(&mut buffer)?;
    Ok(i32::from_be_bytes(buffer))
}

fn read_u32_be<R: Read>(reader: &mut R) -> Result<u32, NbtError> {
    let mut buffer = [0_u8; 4];
    reader.read_exact(&mut buffer)?;
    Ok(u32::from_be_bytes(buffer))
}

fn read_i64_be<R: Read>(reader: &mut R) -> Result<i64, NbtError> {
    let mut buffer = [0_u8; 8];
    reader.read_exact(&mut buffer)?;
    Ok(i64::from_be_bytes(buffer))
}

fn read_u64_be<R: Read>(reader: &mut R) -> Result<u64, NbtError> {
    let mut buffer = [0_u8; 8];
    reader.read_exact(&mut buffer)?;
    Ok(u64::from_be_bytes(buffer))
}

fn write_u8<W: Write>(writer: &mut W, value: u8) -> Result<(), NbtError> {
    writer.write_all(&[value])?;
    Ok(())
}

fn write_i8<W: Write>(writer: &mut W, value: i8) -> Result<(), NbtError> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_u16_be<W: Write>(writer: &mut W, value: u16) -> Result<(), NbtError> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_i16_be<W: Write>(writer: &mut W, value: i16) -> Result<(), NbtError> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_i32_be<W: Write>(writer: &mut W, value: i32) -> Result<(), NbtError> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_u32_be<W: Write>(writer: &mut W, value: u32) -> Result<(), NbtError> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_i64_be<W: Write>(writer: &mut W, value: i64) -> Result<(), NbtError> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_u64_be<W: Write>(writer: &mut W, value: u64) -> Result<(), NbtError> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn len_to_i32(length: usize, context: &'static str) -> Result<i32, NbtError> {
    i32::try_from(length).map_err(|_| NbtError::LengthTooLarge { context, length })
}

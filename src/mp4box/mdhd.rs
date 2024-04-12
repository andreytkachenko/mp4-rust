use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MdhdBox {
    pub version: u8,
    pub flags: u32,
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub language: String,
}

impl MdhdBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MdhdBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;

        if self.version == 1 {
            size += 28;
        } else if self.version == 0 {
            size += 16;
        }
        size += 4;
        size
    }
}

impl Default for MdhdBox {
    fn default() -> Self {
        MdhdBox {
            version: 0,
            flags: 0,
            creation_time: 0,
            modification_time: 0,
            timescale: 1000,
            duration: 0,
            language: String::from("und"),
        }
    }
}

impl Mp4Box for MdhdBox {
    const TYPE: BoxType = BoxType::MdhdBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "creation_time={} timescale={} duration={} language={}",
            self.creation_time, self.timescale, self.duration, self.language
        );
        Ok(s)
    }
}

impl BlockReader for MdhdBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let (creation_time, modification_time, timescale, duration) = if version == 1 {
            (
                reader.get_u64(),
                reader.get_u64(),
                reader.get_u32(),
                reader.get_u64(),
            )
        } else if version == 0 {
            (
                reader.get_u32() as u64,
                reader.get_u32() as u64,
                reader.get_u32(),
                reader.get_u32() as u64,
            )
        } else {
            return Err(Error::InvalidData("version must be 0 or 1"));
        };

        let language_code = reader.get_u16();
        let language = language_string(language_code);

        Ok(MdhdBox {
            version,
            flags,
            creation_time,
            modification_time,
            timescale,
            duration,
            language,
        })
    }

    fn size_hint() -> usize {
        22
    }
}

impl<W: Write> WriteBox<&mut W> for MdhdBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        if self.version == 1 {
            writer.write_u64::<BigEndian>(self.creation_time)?;
            writer.write_u64::<BigEndian>(self.modification_time)?;
            writer.write_u32::<BigEndian>(self.timescale)?;
            writer.write_u64::<BigEndian>(self.duration)?;
        } else if self.version == 0 {
            writer.write_u32::<BigEndian>(self.creation_time as u32)?;
            writer.write_u32::<BigEndian>(self.modification_time as u32)?;
            writer.write_u32::<BigEndian>(self.timescale)?;
            writer.write_u32::<BigEndian>(self.duration as u32)?;
        } else {
            return Err(Error::InvalidData("version must be 0 or 1"));
        }

        let language_code = language_code(&self.language);
        writer.write_u16::<BigEndian>(language_code)?;
        writer.write_u16::<BigEndian>(0)?; // pre-defined

        Ok(size)
    }
}

fn language_string(language: u16) -> String {
    let mut lang: [u16; 3] = [0; 3];

    lang[0] = ((language >> 10) & 0x1F) + 0x60;
    lang[1] = ((language >> 5) & 0x1F) + 0x60;
    lang[2] = ((language) & 0x1F) + 0x60;

    // Decode utf-16 encoded bytes into a string.
    let lang_str = decode_utf16(lang.iter().cloned())
        .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
        .collect::<String>();

    lang_str
}

fn language_code(language: &str) -> u16 {
    let mut lang = language.encode_utf16();
    let mut code = (lang.next().unwrap_or(0) & 0x1F) << 10;
    code += (lang.next().unwrap_or(0) & 0x1F) << 5;
    code += lang.next().unwrap_or(0) & 0x1F;
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    fn test_language_code(lang: &str) {
        let code = language_code(lang);
        let lang2 = language_string(code);
        assert_eq!(lang, lang2);
    }

    #[test]
    fn test_language_codes() {
        test_language_code("und");
        test_language_code("eng");
        test_language_code("kor");
    }

    #[test]
    fn test_mdhd32() {
        let src_box = MdhdBox {
            version: 0,
            flags: 0,
            creation_time: 100,
            modification_time: 200,
            timescale: 48000,
            duration: 30439936,
            language: String::from("und"),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MdhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = MdhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_mdhd64() {
        let src_box = MdhdBox {
            version: 0,
            flags: 0,
            creation_time: 100,
            modification_time: 200,
            timescale: 48000,
            duration: 30439936,
            language: String::from("eng"),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MdhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = MdhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

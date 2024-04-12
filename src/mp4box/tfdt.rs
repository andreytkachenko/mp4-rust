use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct TfdtBox {
    pub version: u8,
    pub flags: u32,
    pub base_media_decode_time: u64,
}

impl TfdtBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::TfdtBox
    }

    pub fn get_size(&self) -> u64 {
        let mut sum = HEADER_SIZE + HEADER_EXT_SIZE;
        if self.version == 1 {
            sum += 8;
        } else {
            sum += 4;
        }
        sum
    }
}

impl Mp4Box for TfdtBox {
    const TYPE: BoxType = BoxType::TfdtBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("base_media_decode_time={}", self.base_media_decode_time);
        Ok(s)
    }
}

impl BlockReader for TfdtBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let base_media_decode_time = if version == 1 {
            reader.get_u64()
        } else if version == 0 {
            reader.get_u32() as u64
        } else {
            return Err(Error::InvalidData("version must be 0 or 1"));
        };

        Ok(TfdtBox {
            version,
            flags,
            base_media_decode_time,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for TfdtBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        if self.version == 1 {
            writer.write_u64::<BigEndian>(self.base_media_decode_time)?;
        } else if self.version == 0 {
            writer.write_u32::<BigEndian>(self.base_media_decode_time as u32)?;
        } else {
            return Err(Error::InvalidData("version must be 0 or 1"));
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_tfdt32() {
        let src_box = TfdtBox {
            version: 0,
            flags: 0,
            base_media_decode_time: 0,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::TfdtBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = TfdtBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_tfdt64() {
        let src_box = TfdtBox {
            version: 1,
            flags: 0,
            base_media_decode_time: 0,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::TfdtBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = TfdtBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

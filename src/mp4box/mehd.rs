use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct MehdBox {
    pub version: u8,
    pub flags: u32,
    pub fragment_duration: u64,
}

impl MehdBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MehdBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;

        if self.version == 1 {
            size += 8;
        } else if self.version == 0 {
            size += 4;
        }
        size
    }
}

impl Mp4Box for MehdBox {
    const TYPE: BoxType = BoxType::MehdBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("fragment_duration={}", self.fragment_duration);
        Ok(s)
    }
}

impl BlockReader for MehdBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let fragment_duration = if version == 1 {
            reader.get_u64()
        } else if version == 0 {
            reader.get_u32() as u64
        } else {
            return Err(Error::InvalidData("version must be 0 or 1"));
        };

        Ok(MehdBox {
            version,
            flags,
            fragment_duration,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for MehdBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        if self.version == 1 {
            writer.write_u64::<BigEndian>(self.fragment_duration)?;
        } else if self.version == 0 {
            writer.write_u32::<BigEndian>(self.fragment_duration as u32)?;
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
    fn test_mehd32() {
        let src_box = MehdBox {
            version: 0,
            flags: 0,
            fragment_duration: 32,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MehdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = MehdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_mehd64() {
        let src_box = MehdBox {
            version: 0,
            flags: 0,
            fragment_duration: 30439936,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MehdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = MehdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

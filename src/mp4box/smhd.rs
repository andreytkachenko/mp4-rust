use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SmhdBox {
    pub version: u8,
    pub flags: u32,

    #[serde(with = "value_i16")]
    pub balance: FixedPointI8,
}

impl SmhdBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::SmhdBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4
    }
}

impl Default for SmhdBox {
    fn default() -> Self {
        SmhdBox {
            version: 0,
            flags: 0,
            balance: FixedPointI8::new_raw(0),
        }
    }
}

impl Mp4Box for SmhdBox {
    const TYPE: BoxType = BoxType::SmhdBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("balance={}", self.balance.value());
        Ok(s)
    }
}

impl BlockReader for SmhdBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        Ok(SmhdBox {
            version,
            flags,
            balance: FixedPointI8::new_raw(reader.get_i16()),
        })
    }

    fn size_hint() -> usize {
        6
    }
}

impl<W: Write> WriteBox<&mut W> for SmhdBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_i16::<BigEndian>(self.balance.raw_value())?;
        writer.write_u16::<BigEndian>(0)?; // reserved

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_smhd() {
        let src_box = SmhdBox {
            version: 0,
            flags: 0,
            balance: FixedPointI8::new_raw(-1),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::SmhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = SmhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

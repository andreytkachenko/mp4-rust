use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MfhdBox {
    pub version: u8,
    pub flags: u32,
    pub sequence_number: u32,
}

impl Default for MfhdBox {
    fn default() -> Self {
        MfhdBox {
            version: 0,
            flags: 0,
            sequence_number: 1,
        }
    }
}

impl MfhdBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MfhdBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4
    }
}

impl Mp4Box for MfhdBox {
    const TYPE: BoxType = BoxType::MfhdBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("sequence_number={}", self.sequence_number);
        Ok(s)
    }
}

impl BlockReader for MfhdBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        Ok(MfhdBox {
            version,
            flags,
            sequence_number: reader.get_u32(),
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for MfhdBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;
        writer.write_u32::<BigEndian>(self.sequence_number)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_mfhd() {
        let src_box = MfhdBox {
            version: 0,
            flags: 0,
            sequence_number: 1,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MfhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = MfhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

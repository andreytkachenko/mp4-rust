use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;
use std::mem::size_of;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct StcoBox {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<u32>,
}

impl StcoBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::StcoBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (4 * self.entries.len() as u64)
    }
}

impl Mp4Box for StcoBox {
    const TYPE: BoxType = BoxType::StcoBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("entries={}", self.entries.len());
        Ok(s)
    }
}

impl BlockReader for StcoBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let entry_size = size_of::<u32>(); // chunk_offset
        let entry_count = reader.get_u32();
        if entry_count as usize > reader.remaining() / entry_size {
            return Err(Error::InvalidData(
                "stco entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _i in 0..entry_count {
            let chunk_offset = reader.get_u32();
            entries.push(chunk_offset);
        }

        Ok(StcoBox {
            version,
            flags,
            entries,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for StcoBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for chunk_offset in self.entries.iter() {
            writer.write_u32::<BigEndian>(*chunk_offset)?;
        }

        Ok(size)
    }
}

impl std::convert::TryFrom<&co64::Co64Box> for StcoBox {
    type Error = std::num::TryFromIntError;

    fn try_from(co64: &co64::Co64Box) -> std::result::Result<Self, Self::Error> {
        let entries = co64
            .entries
            .iter()
            .copied()
            .map(u32::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Self {
            version: 0,
            flags: 0,
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_stco() {
        let src_box = StcoBox {
            version: 0,
            flags: 0,
            entries: vec![267, 1970, 2535, 2803, 11843, 22223, 33584],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::StcoBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = StcoBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

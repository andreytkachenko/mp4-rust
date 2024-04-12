use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;
use std::mem::size_of;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct StssBox {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<u32>,
}

impl StssBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::StssBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (4 * self.entries.len() as u64)
    }
}

impl Mp4Box for StssBox {
    const TYPE: BoxType = BoxType::StssBox;

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

impl BlockReader for StssBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let entry_size = size_of::<u32>(); // sample_number
        let entry_count = reader.get_u32();
        if entry_count as usize > reader.remaining() / entry_size {
            return Err(Error::InvalidData(
                "stss entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _i in 0..entry_count {
            let sample_number = reader.get_u32();
            entries.push(sample_number);
        }

        Ok(StssBox {
            version,
            flags,
            entries,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for StssBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for sample_number in self.entries.iter() {
            writer.write_u32::<BigEndian>(*sample_number)?;
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_stss() {
        let src_box = StssBox {
            version: 0,
            flags: 0,
            entries: vec![1, 61, 121, 181, 241, 301, 361, 421, 481],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::StssBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = StssBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;
use std::mem::size_of;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct SttsBox {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<SttsEntry>,
}

impl SttsBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::SttsBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (8 * self.entries.len() as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct SttsEntry {
    pub sample_count: u32,
    pub sample_delta: u32,
}

impl Mp4Box for SttsBox {
    const TYPE: BoxType = BoxType::SttsBox;

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

impl BlockReader for SttsBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let entry_size = size_of::<u32>() + size_of::<u32>(); // sample_count + sample_delta
        let entry_count = reader.get_u32();

        if entry_count as usize > reader.remaining() / entry_size {
            return Err(Error::InvalidData(
                "stts entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _i in 0..entry_count {
            let entry = SttsEntry {
                sample_count: reader.get_u32(),
                sample_delta: reader.get_u32(),
            };
            entries.push(entry);
        }

        Ok(SttsBox {
            version,
            flags,
            entries,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for SttsBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for entry in self.entries.iter() {
            writer.write_u32::<BigEndian>(entry.sample_count)?;
            writer.write_u32::<BigEndian>(entry.sample_delta)?;
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_stts() {
        let src_box = SttsBox {
            version: 0,
            flags: 0,
            entries: vec![
                SttsEntry {
                    sample_count: 29726,
                    sample_delta: 1024,
                },
                SttsEntry {
                    sample_count: 1,
                    sample_delta: 512,
                },
            ],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::SttsBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = SttsBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

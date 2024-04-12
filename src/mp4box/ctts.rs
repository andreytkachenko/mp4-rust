use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;
use std::mem::size_of;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct CttsBox {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<CttsEntry>,
}

impl CttsBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::CttsBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (8 * self.entries.len() as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct CttsEntry {
    pub sample_count: u32,
    pub sample_offset: i32,
}

impl Mp4Box for CttsBox {
    const TYPE: BoxType = BoxType::CttsBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("entries_count={}", self.entries.len());
        Ok(s)
    }
}

impl BlockReader for CttsBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let entry_count = reader.get_u32();
        let entry_size = size_of::<u32>() + size_of::<i32>(); // sample_count + sample_offset
                                                              // (sample_offset might be a u32, but the size is the same.)

        if entry_count as usize > reader.remaining() / entry_size {
            return Err(Error::InvalidData(
                "ctts entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let entry = CttsEntry {
                sample_count: reader.get_u32(),
                sample_offset: reader.get_i32(),
            };
            entries.push(entry);
        }

        Ok(CttsBox {
            version,
            flags,
            entries,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for CttsBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for entry in self.entries.iter() {
            writer.write_u32::<BigEndian>(entry.sample_count)?;
            writer.write_i32::<BigEndian>(entry.sample_offset)?;
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_ctts() {
        let src_box = CttsBox {
            version: 0,
            flags: 0,
            entries: vec![
                CttsEntry {
                    sample_count: 1,
                    sample_offset: 200,
                },
                CttsEntry {
                    sample_count: 2,
                    sample_offset: -100,
                },
            ],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::CttsBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = CttsBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

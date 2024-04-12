use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;
use std::mem::size_of;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ElstBox {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<ElstEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ElstEntry {
    pub segment_duration: u64,
    pub media_time: u64,
    pub media_rate: u16,
    pub media_rate_fraction: u16,
}

impl ElstBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::ElstBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE + 4;
        if self.version == 1 {
            size += self.entries.len() as u64 * 20;
        } else if self.version == 0 {
            size += self.entries.len() as u64 * 12;
        }
        size
    }
}

impl Mp4Box for ElstBox {
    const TYPE: BoxType = BoxType::ElstBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("elst_entries={}", self.entries.len());
        Ok(s)
    }
}

impl BlockReader for ElstBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let entry_count = reader.get_u32();
        let entry_size = {
            let mut entry_size = 0;
            entry_size += if version == 1 {
                size_of::<u64>() + size_of::<i64>() // segment_duration + media_time
            } else {
                size_of::<u32>() + size_of::<i32>() // segment_duration + media_time
            };

            entry_size += size_of::<i16>() + size_of::<i16>(); // media_rate_integer + media_rate_fraction
            entry_size
        };

        if entry_count as usize > reader.remaining() / entry_size {
            return Err(Error::InvalidData(
                "elst entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let (segment_duration, media_time) = if version == 1 {
                (reader.get_u64(), reader.get_u64())
            } else {
                (reader.get_u32() as u64, reader.get_u32() as u64)
            };

            entries.push(ElstEntry {
                segment_duration,
                media_time,
                media_rate: reader.get_u16(),
                media_rate_fraction: reader.get_u16(),
            });
        }

        Ok(ElstBox {
            version,
            flags,
            entries,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for ElstBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for entry in self.entries.iter() {
            if self.version == 1 {
                writer.write_u64::<BigEndian>(entry.segment_duration)?;
                writer.write_u64::<BigEndian>(entry.media_time)?;
            } else {
                writer.write_u32::<BigEndian>(entry.segment_duration as u32)?;
                writer.write_u32::<BigEndian>(entry.media_time as u32)?;
            }
            writer.write_u16::<BigEndian>(entry.media_rate)?;
            writer.write_u16::<BigEndian>(entry.media_rate_fraction)?;
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_elst32() {
        let src_box = ElstBox {
            version: 0,
            flags: 0,
            entries: vec![ElstEntry {
                segment_duration: 634634,
                media_time: 0,
                media_rate: 1,
                media_rate_fraction: 0,
            }],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::ElstBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = ElstBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_elst64() {
        let src_box = ElstBox {
            version: 1,
            flags: 0,
            entries: vec![ElstEntry {
                segment_duration: 634634,
                media_time: 0,
                media_rate: 1,
                media_rate_fraction: 0,
            }],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::ElstBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = ElstBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

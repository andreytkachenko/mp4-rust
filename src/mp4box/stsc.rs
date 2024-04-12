use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;
use std::mem::size_of;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct StscBox {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<StscEntry>,
}

impl StscBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::StscBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (12 * self.entries.len() as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct StscEntry {
    pub first_chunk: u32,
    pub samples_per_chunk: u32,
    pub sample_description_index: u32,
    pub first_sample: u32,
}

impl Mp4Box for StscBox {
    const TYPE: BoxType = BoxType::StscBox;

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

impl BlockReader for StscBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let entry_size = size_of::<u32>() + size_of::<u32>() + size_of::<u32>(); // first_chunk + samples_per_chunk + sample_description_index
        let entry_count = reader.get_u32();
        if entry_count as usize > reader.remaining() / entry_size {
            return Err(Error::InvalidData(
                "stsc entry_count indicates more entries than could fit in the box",
            ));
        }
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let entry = StscEntry {
                first_chunk: reader.get_u32(),
                samples_per_chunk: reader.get_u32(),
                sample_description_index: reader.get_u32(),
                first_sample: 0,
            };
            entries.push(entry);
        }

        let mut sample_id = 1;
        for i in 0..entry_count {
            let (first_chunk, samples_per_chunk) = {
                let entry = entries.get_mut(i as usize).unwrap();
                entry.first_sample = sample_id;
                (entry.first_chunk, entry.samples_per_chunk)
            };
            if i < entry_count - 1 {
                let next_entry = entries.get(i as usize + 1).unwrap();
                sample_id = next_entry
                    .first_chunk
                    .checked_sub(first_chunk)
                    .and_then(|n| n.checked_mul(samples_per_chunk))
                    .and_then(|n| n.checked_add(sample_id))
                    .ok_or(Error::InvalidData(
                        "attempt to calculate stsc sample_id with overflow",
                    ))?;
            }
        }

        Ok(StscBox {
            version,
            flags,
            entries,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for StscBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for entry in self.entries.iter() {
            writer.write_u32::<BigEndian>(entry.first_chunk)?;
            writer.write_u32::<BigEndian>(entry.samples_per_chunk)?;
            writer.write_u32::<BigEndian>(entry.sample_description_index)?;
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_stsc() {
        let src_box = StscBox {
            version: 0,
            flags: 0,
            entries: vec![
                StscEntry {
                    first_chunk: 1,
                    samples_per_chunk: 1,
                    sample_description_index: 1,
                    first_sample: 1,
                },
                StscEntry {
                    first_chunk: 19026,
                    samples_per_chunk: 14,
                    sample_description_index: 1,
                    first_sample: 19026,
                },
            ],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::StscBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = StscBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

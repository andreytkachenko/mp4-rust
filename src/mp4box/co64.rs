use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;
use std::mem::size_of;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Co64Box {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing)]
    pub entries: Vec<u64>,
}

impl Co64Box {
    pub fn get_type(&self) -> BoxType {
        BoxType::Co64Box
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 4 + (8 * self.entries.len() as u64)
    }
}

impl Mp4Box for Co64Box {
    const TYPE: BoxType = BoxType::Co64Box;

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

impl BlockReader for Co64Box {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let entry_size = size_of::<u64>(); // chunk_offset
        let entry_count = reader.get_u32();
        if entry_count as usize > reader.remaining() / entry_size {
            return Err(Error::InvalidData(
                "co64 entry_count indicates more entries than could fit in the box",
            ));
        }

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _i in 0..entry_count {
            let chunk_offset = reader.get_u64();
            entries.push(chunk_offset);
        }

        Ok(Co64Box {
            version,
            flags,
            entries,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for Co64Box {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;
        for chunk_offset in self.entries.iter() {
            writer.write_u64::<BigEndian>(*chunk_offset)?;
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_co64() {
        let src_box = Co64Box {
            version: 0,
            flags: 0,
            entries: vec![267, 1970, 2535, 2803, 11843, 22223, 33584],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let header = BoxHeader::read_sync(&mut buf.as_slice()).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::Co64Box);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = Co64Box::read_block(&mut buf.as_slice()).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

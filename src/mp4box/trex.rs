use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct TrexBox {
    pub version: u8,
    pub flags: u32,
    pub track_id: u32,
    pub default_sample_description_index: u32,
    pub default_sample_duration: u32,
    pub default_sample_size: u32,
    pub default_sample_flags: u32,
}

impl TrexBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::TrexBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 20
    }
}

impl Mp4Box for TrexBox {
    const TYPE: BoxType = BoxType::TrexBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "track_id={} default_sample_duration={}",
            self.track_id, self.default_sample_duration
        );
        Ok(s)
    }
}

impl BlockReader for TrexBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let track_id = reader.get_u32();
        let default_sample_description_index = reader.get_u32();
        let default_sample_duration = reader.get_u32();
        let default_sample_size = reader.get_u32();
        let default_sample_flags = reader.get_u32();

        Ok(TrexBox {
            version,
            flags,
            track_id,
            default_sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        })
    }

    fn size_hint() -> usize {
        24
    }
}

impl<W: Write> WriteBox<&mut W> for TrexBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(self.track_id)?;
        writer.write_u32::<BigEndian>(self.default_sample_description_index)?;
        writer.write_u32::<BigEndian>(self.default_sample_duration)?;
        writer.write_u32::<BigEndian>(self.default_sample_size)?;
        writer.write_u32::<BigEndian>(self.default_sample_flags)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_trex() {
        let src_box = TrexBox {
            version: 0,
            flags: 0,
            track_id: 1,
            default_sample_description_index: 1,
            default_sample_duration: 1000,
            default_sample_size: 0,
            default_sample_flags: 65536,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::TrexBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = TrexBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

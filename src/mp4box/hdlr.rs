use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct HdlrBox {
    pub version: u8,
    pub flags: u32,
    pub handler_type: FourCC,
    pub name: String,
}

impl HdlrBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::HdlrBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 20 + self.name.len() as u64 + 1
    }
}

impl Mp4Box for HdlrBox {
    const TYPE: BoxType = BoxType::HdlrBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("handler_type={} name={}", self.handler_type, self.name);
        Ok(s)
    }
}

impl BlockReader for HdlrBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        reader.get_u32(); // pre-defined

        let handler = reader.get_u32();

        reader.skip(12);

        Ok(HdlrBox {
            version,
            flags,
            handler_type: From::from(handler),
            name: reader.get_null_terminated_string(),
        })
    }

    fn size_hint() -> usize {
        24
    }
}

impl<W: Write> WriteBox<&mut W> for HdlrBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(0)?; // pre-defined
        writer.write_u32::<BigEndian>((&self.handler_type).into())?;

        // 12 bytes reserved
        for _ in 0..3 {
            writer.write_u32::<BigEndian>(0)?;
        }

        writer.write_all(self.name.as_bytes())?;
        writer.write_u8(0)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_hdlr() {
        let src_box = HdlrBox {
            version: 0,
            flags: 0,
            handler_type: str::parse::<FourCC>("vide").unwrap(),
            name: String::from("VideoHandler"),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::HdlrBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = HdlrBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_hdlr_empty() {
        let src_box = HdlrBox {
            version: 0,
            flags: 0,
            handler_type: str::parse::<FourCC>("vide").unwrap(),
            name: String::new(),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::HdlrBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = HdlrBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_hdlr_extra() {
        let real_src_box = HdlrBox {
            version: 0,
            flags: 0,
            handler_type: str::parse::<FourCC>("vide").unwrap(),
            name: String::from("Good"),
        };
        let src_box = HdlrBox {
            version: 0,
            flags: 0,
            handler_type: str::parse::<FourCC>("vide").unwrap(),
            name: String::from_utf8(b"Good\0Bad".to_vec()).unwrap(),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::HdlrBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = HdlrBox::read_block(&mut reader).unwrap();
        assert_eq!(real_src_box, dst_box);
    }
}

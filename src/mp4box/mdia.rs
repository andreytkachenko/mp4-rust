use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;
use crate::mp4box::{hdlr::HdlrBox, mdhd::MdhdBox, minf::MinfBox};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct MdiaBox {
    pub mdhd: MdhdBox,
    pub hdlr: HdlrBox,
    pub minf: MinfBox,
}

impl MdiaBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MdiaBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + self.mdhd.box_size() + self.hdlr.box_size() + self.minf.box_size()
    }
}

impl Mp4Box for MdiaBox {
    const TYPE: BoxType = BoxType::MdiaBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = String::new();
        Ok(s)
    }
}

impl BlockReader for MdiaBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (mdhd, hdlr, minf) = reader.find_box3()?;
        Ok(MdiaBox { mdhd, hdlr, minf })
    }

    fn size_hint() -> usize {
        0
    }
}

impl<W: Write> WriteBox<&mut W> for MdiaBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        self.mdhd.write_box(writer)?;
        self.hdlr.write_box(writer)?;
        self.minf.write_box(writer)?;

        Ok(size)
    }
}

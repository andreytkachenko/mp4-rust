use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;
use crate::mp4box::{dinf::DinfBox, smhd::SmhdBox, stbl::StblBox, vmhd::VmhdBox};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct MinfBox {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmhd: Option<VmhdBox>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub smhd: Option<SmhdBox>,

    pub dinf: DinfBox,
    pub stbl: StblBox,
}

impl MinfBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MinfBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        if let Some(ref vmhd) = self.vmhd {
            size += vmhd.box_size();
        }
        if let Some(ref smhd) = self.smhd {
            size += smhd.box_size();
        }
        size += self.dinf.box_size();
        size += self.stbl.box_size();
        size
    }
}

impl Mp4Box for MinfBox {
    const TYPE: BoxType = BoxType::MinfBox;

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

impl BlockReader for MinfBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (vmhd, smhd, dinf, stbl) = reader.try_find_box4()?;

        if dinf.is_none() {
            return Err(Error::BoxNotFound(BoxType::DinfBox));
        }

        if stbl.is_none() {
            return Err(Error::BoxNotFound(BoxType::StblBox));
        }

        Ok(MinfBox {
            vmhd,
            smhd,
            dinf: dinf.unwrap(),
            stbl: stbl.unwrap(),
        })
    }

    fn size_hint() -> usize {
        DinfBox::size_hint() + StblBox::size_hint()
    }
}

impl<W: Write> WriteBox<&mut W> for MinfBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        if let Some(ref vmhd) = self.vmhd {
            vmhd.write_box(writer)?;
        }
        if let Some(ref smhd) = self.smhd {
            smhd.write_box(writer)?;
        }
        self.dinf.write_box(writer)?;
        self.stbl.write_box(writer)?;

        Ok(size)
    }
}

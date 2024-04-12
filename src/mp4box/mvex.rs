use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;
use crate::mp4box::{mehd::MehdBox, trex::TrexBox};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct MvexBox {
    pub mehd: Option<MehdBox>,
    pub trex: TrexBox,
}

impl MvexBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MdiaBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + self.mehd.as_ref().map(|x| x.box_size()).unwrap_or(0) + self.trex.box_size()
    }
}

impl Mp4Box for MvexBox {
    const TYPE: BoxType = BoxType::MvexBox;

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

impl BlockReader for MvexBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (mehd, trex) = reader.try_find_box2::<MehdBox, TrexBox>()?;

        if trex.is_none() {
            return Err(Error::BoxNotFound(BoxType::TrexBox));
        }

        Ok(MvexBox {
            mehd,
            trex: trex.unwrap(),
        })
    }

    fn size_hint() -> usize {
        TrexBox::size_hint()
    }
}

impl<W: Write> WriteBox<&mut W> for MvexBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        if let Some(mehd) = &self.mehd {
            mehd.write_box(writer)?;
        }

        self.trex.write_box(writer)?;

        Ok(size)
    }
}

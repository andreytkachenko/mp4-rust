use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;
use crate::mp4box::{tfdt::TfdtBox, tfhd::TfhdBox, trun::TrunBox};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct TrafBox {
    pub tfhd: TfhdBox,
    pub tfdt: Option<TfdtBox>,
    pub trun: Option<TrunBox>,
}

impl TrafBox {
    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        size += self.tfhd.box_size();
        if let Some(ref tfdt) = self.tfdt {
            size += tfdt.box_size();
        }
        if let Some(ref trun) = self.trun {
            size += trun.box_size();
        }
        size
    }
}

impl Mp4Box for TrafBox {
    const TYPE: BoxType = BoxType::TrafBox;

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

impl BlockReader for TrafBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (tfhd, tfdt, trun) = reader.try_find_box3()?;

        if tfhd.is_none() {
            return Err(Error::BoxNotFound(BoxType::TfhdBox));
        }

        Ok(TrafBox {
            tfhd: tfhd.unwrap(),
            tfdt,
            trun,
        })
    }

    fn size_hint() -> usize {
        TfhdBox::size_hint()
    }
}

impl<W: Write> WriteBox<&mut W> for TrafBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        self.tfhd.write_box(writer)?;
        if let Some(ref tfdt) = self.tfdt {
            tfdt.write_box(writer)?;
        }
        if let Some(ref trun) = self.trun {
            trun.write_box(writer)?;
        }

        Ok(size)
    }
}

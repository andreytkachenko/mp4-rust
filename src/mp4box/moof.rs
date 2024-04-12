use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;
use crate::mp4box::{mfhd::MfhdBox, traf::TrafBox};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct MoofBox {
    pub mfhd: MfhdBox,

    #[serde(rename = "traf")]
    pub trafs: Vec<TrafBox>,
}

impl MoofBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MoofBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + self.mfhd.box_size();
        for traf in self.trafs.iter() {
            size += traf.box_size();
        }
        size
    }
}

impl Mp4Box for MoofBox {
    const TYPE: BoxType = BoxType::MoofBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("trafs={}", self.trafs.len());
        Ok(s)
    }
}

impl BlockReader for MoofBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let mut mfhd = None;
        let mut trafs = Vec::new();

        while let Some(mut bx) = reader.get_box()? {
            match bx.kind {
                BoxType::MfhdBox => {
                    mfhd = Some(bx.read()?);
                }

                BoxType::TrafBox => {
                    trafs.push(bx.read()?);
                }

                _ => continue,
            }
        }

        if mfhd.is_none() {
            return Err(Error::BoxNotFound(BoxType::MfhdBox));
        }

        Ok(MoofBox {
            mfhd: mfhd.unwrap(),
            trafs,
        })
    }

    fn size_hint() -> usize {
        MfhdBox::size_hint()
    }
}

impl<W: Write> WriteBox<&mut W> for MoofBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        self.mfhd.write_box(writer)?;
        for traf in self.trafs.iter() {
            traf.write_box(writer)?;
        }
        Ok(0)
    }
}

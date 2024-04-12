use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;
use crate::mp4box::{
    co64::Co64Box, ctts::CttsBox, stco::StcoBox, stsc::StscBox, stsd::StsdBox, stss::StssBox,
    stsz::StszBox, stts::SttsBox,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct StblBox {
    pub stsd: StsdBox,
    pub stts: SttsBox,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctts: Option<CttsBox>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stss: Option<StssBox>,
    pub stsc: StscBox,
    pub stsz: StszBox,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stco: Option<StcoBox>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub co64: Option<Co64Box>,
}

impl StblBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::StblBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        size += self.stsd.box_size();
        size += self.stts.box_size();
        if let Some(ref ctts) = self.ctts {
            size += ctts.box_size();
        }
        if let Some(ref stss) = self.stss {
            size += stss.box_size();
        }
        size += self.stsc.box_size();
        size += self.stsz.box_size();
        if let Some(ref stco) = self.stco {
            size += stco.box_size();
        }
        if let Some(ref co64) = self.co64 {
            size += co64.box_size();
        }
        size
    }
}

impl Mp4Box for StblBox {
    const TYPE: BoxType = BoxType::StblBox;

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

impl BlockReader for StblBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let mut stsd = None;
        let mut stts = None;
        let mut ctts = None;
        let mut stss = None;
        let mut stsc = None;
        let mut stsz = None;
        let mut stco = None;
        let mut co64 = None;

        while let Some(mut bx) = reader.get_box()? {
            match bx.kind {
                BoxType::StsdBox => {
                    stsd = Some(bx.read()?);
                }

                BoxType::SttsBox => {
                    stts = Some(bx.read()?);
                }

                BoxType::CttsBox => {
                    ctts = Some(bx.read()?);
                }

                BoxType::StssBox => {
                    stss = Some(bx.read()?);
                }

                BoxType::StscBox => {
                    stsc = Some(bx.read()?);
                }

                BoxType::StszBox => {
                    stsz = Some(bx.read()?);
                }

                BoxType::StcoBox => {
                    stco = Some(bx.read()?);
                }

                BoxType::Co64Box => {
                    co64 = Some(bx.read()?);
                }

                _ => continue,
            }
        }

        if stsd.is_none() {
            return Err(Error::BoxNotFound(BoxType::StsdBox));
        }

        if stts.is_none() {
            return Err(Error::BoxNotFound(BoxType::SttsBox));
        }

        if stsc.is_none() {
            return Err(Error::BoxNotFound(BoxType::StscBox));
        }

        if stsz.is_none() {
            return Err(Error::BoxNotFound(BoxType::StszBox));
        }

        if stco.is_none() && co64.is_none() {
            return Err(Error::Box2NotFound(BoxType::StcoBox, BoxType::Co64Box));
        }

        Ok(StblBox {
            stsd: stsd.unwrap(),
            stts: stts.unwrap(),
            ctts,
            stss,
            stsc: stsc.unwrap(),
            stsz: stsz.unwrap(),
            stco,
            co64,
        })
    }

    fn size_hint() -> usize {
        StsdBox::size_hint() + SttsBox::size_hint() + StscBox::size_hint() + StszBox::size_hint()
    }
}

impl<W: Write> WriteBox<&mut W> for StblBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        self.stsd.write_box(writer)?;
        self.stts.write_box(writer)?;
        if let Some(ref ctts) = self.ctts {
            ctts.write_box(writer)?;
        }
        if let Some(ref stss) = self.stss {
            stss.write_box(writer)?;
        }
        self.stsc.write_box(writer)?;
        self.stsz.write_box(writer)?;
        if let Some(ref stco) = self.stco {
            stco.write_box(writer)?;
        }
        if let Some(ref co64) = self.co64 {
            co64.write_box(writer)?;
        }

        Ok(size)
    }
}

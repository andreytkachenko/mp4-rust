use serde::Serialize;
use std::io::Write;

use crate::mp4box::elst::ElstBox;
use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct EdtsBox {
    pub elst: Option<ElstBox>,
}

impl EdtsBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::EdtsBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        if let Some(ref elst) = self.elst {
            size += elst.box_size();
        }
        size
    }
}

impl Mp4Box for EdtsBox {
    const TYPE: BoxType = BoxType::EdtsBox;

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

impl BlockReader for EdtsBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        Ok(EdtsBox {
            elst: reader.try_find_box::<ElstBox>()?,
        })
    }

    fn size_hint() -> usize {
        0
    }
}

impl<W: Write> WriteBox<&mut W> for EdtsBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        if let Some(ref elst) = self.elst {
            elst.write_box(writer)?;
        }

        Ok(size)
    }
}

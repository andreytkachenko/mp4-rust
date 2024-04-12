use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct VmhdBox {
    pub version: u8,
    pub flags: u32,
    pub graphics_mode: u16,
    pub op_color: RgbColor,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct RgbColor {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

impl VmhdBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::VmhdBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE + 8
    }
}

impl Mp4Box for VmhdBox {
    const TYPE: BoxType = BoxType::VmhdBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "graphics_mode={} op_color={}{}{}",
            self.graphics_mode, self.op_color.red, self.op_color.green, self.op_color.blue
        );
        Ok(s)
    }
}

impl BlockReader for VmhdBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);
        let graphics_mode = reader.get_u16();
        let op_color = RgbColor {
            red: reader.get_u16(),
            green: reader.get_u16(),
            blue: reader.get_u16(),
        };

        Ok(VmhdBox {
            version,
            flags,
            graphics_mode,
            op_color,
        })
    }

    fn size_hint() -> usize {
        12
    }
}

impl<W: Write> WriteBox<&mut W> for VmhdBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u16::<BigEndian>(self.graphics_mode)?;
        writer.write_u16::<BigEndian>(self.op_color.red)?;
        writer.write_u16::<BigEndian>(self.op_color.green)?;
        writer.write_u16::<BigEndian>(self.op_color.blue)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_vmhd() {
        let src_box = VmhdBox {
            version: 0,
            flags: 1,
            graphics_mode: 0,
            op_color: RgbColor {
                red: 0,
                green: 0,
                blue: 0,
            },
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::VmhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = VmhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

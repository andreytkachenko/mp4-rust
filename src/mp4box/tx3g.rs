use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tx3gBox {
    pub data_reference_index: u16,
    pub display_flags: u32,
    pub horizontal_justification: i8,
    pub vertical_justification: i8,
    pub bg_color_rgba: RgbaColor,
    pub box_record: [i16; 4],
    pub style_record: [u8; 12],
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct RgbaColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Default for Tx3gBox {
    fn default() -> Self {
        Tx3gBox {
            data_reference_index: 0,
            display_flags: 0,
            horizontal_justification: 1,
            vertical_justification: -1,
            bg_color_rgba: RgbaColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            },
            box_record: [0, 0, 0, 0],
            style_record: [0, 0, 0, 0, 0, 1, 0, 16, 255, 255, 255, 255],
        }
    }
}

impl Tx3gBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::Tx3gBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + 6 + 32
    }
}

impl Mp4Box for Tx3gBox {
    const TYPE: BoxType = BoxType::Tx3gBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("data_reference_index={} horizontal_justification={} vertical_justification={} rgba={}{}{}{}",
            self.data_reference_index, self.horizontal_justification,
            self.vertical_justification, self.bg_color_rgba.red,
            self.bg_color_rgba.green, self.bg_color_rgba.blue, self.bg_color_rgba.alpha);
        Ok(s)
    }
}

impl BlockReader for Tx3gBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        reader.get_u32(); // reserved
        reader.get_u16(); // reserved
        let data_reference_index = reader.get_u16();

        let display_flags = reader.get_u32();
        let horizontal_justification = reader.get_i8();
        let vertical_justification = reader.get_i8();
        let bg_color_rgba = RgbaColor {
            red: reader.get_u8(),
            green: reader.get_u8(),
            blue: reader.get_u8(),
            alpha: reader.get_u8(),
        };
        let box_record: [i16; 4] = [
            reader.get_i16(),
            reader.get_i16(),
            reader.get_i16(),
            reader.get_i16(),
        ];
        let style_record: [u8; 12] = [
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
            reader.get_u8(),
        ];

        Ok(Tx3gBox {
            data_reference_index,
            display_flags,
            horizontal_justification,
            vertical_justification,
            bg_color_rgba,
            box_record,
            style_record,
        })
    }

    fn size_hint() -> usize {
        34
    }
}

impl<W: Write> WriteBox<&mut W> for Tx3gBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        writer.write_u32::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.data_reference_index)?;
        writer.write_u32::<BigEndian>(self.display_flags)?;
        writer.write_i8(self.horizontal_justification)?;
        writer.write_i8(self.vertical_justification)?;
        writer.write_u8(self.bg_color_rgba.red)?;
        writer.write_u8(self.bg_color_rgba.green)?;
        writer.write_u8(self.bg_color_rgba.blue)?;
        writer.write_u8(self.bg_color_rgba.alpha)?;
        for n in 0..4 {
            writer.write_i16::<BigEndian>(self.box_record[n])?;
        }
        for n in 0..12 {
            writer.write_u8(self.style_record[n])?;
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_tx3g() {
        let src_box = Tx3gBox {
            data_reference_index: 1,
            display_flags: 0,
            horizontal_justification: 1,
            vertical_justification: -1,
            bg_color_rgba: RgbaColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            },
            box_record: [0, 0, 0, 0],
            style_record: [0, 0, 0, 0, 0, 1, 0, 16, 255, 255, 255, 255],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::Tx3gBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = Tx3gBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

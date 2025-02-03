use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

pub enum TrackFlag {
    TrackEnabled = 0x000001,
    // TrackInMovie = 0x000002,
    // TrackInPreview = 0x000004,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TkhdBox {
    pub version: u8,
    pub flags: u32,
    pub creation_time: u64,
    pub modification_time: u64,
    pub track_id: u32,
    pub duration: u64,
    pub layer: u16,
    pub alternate_group: u16,

    #[serde(with = "value_u8")]
    pub volume: FixedPointU8,
    pub matrix: Matrix,

    #[serde(with = "value_u32")]
    pub width: FixedPointU16,

    #[serde(with = "value_u32")]
    pub height: FixedPointU16,
}

impl Default for TkhdBox {
    fn default() -> Self {
        TkhdBox {
            version: 0,
            flags: TrackFlag::TrackEnabled as u32,
            creation_time: 0,
            modification_time: 0,
            track_id: 0,
            duration: 0,
            layer: 0,
            alternate_group: 0,
            volume: FixedPointU8::new(1),
            matrix: Matrix::default(),
            width: FixedPointU16::new(0),
            height: FixedPointU16::new(0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Matrix {
    pub a: i32,
    pub b: i32,
    pub u: i32,
    pub c: i32,
    pub d: i32,
    pub v: i32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
}

impl std::fmt::Display for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}",
            self.a, self.b, self.u, self.c, self.d, self.v, self.x, self.y, self.w
        )
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self {
            // unity matrix according to ISO/IEC 14496-12:2005(E)
            a: 0x00010000,
            b: 0,
            u: 0,
            c: 0,
            d: 0x00010000,
            v: 0,
            x: 0,
            y: 0,
            w: 0x40000000,
        }
    }
}

impl TkhdBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::TkhdBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;
        if self.version == 1 {
            size += 32;
        } else if self.version == 0 {
            size += 20;
        }
        size += 60;
        size
    }

    pub fn set_width(&mut self, width: u16) {
        self.width = FixedPointU16::new(width);
    }

    pub fn set_height(&mut self, height: u16) {
        self.height = FixedPointU16::new(height);
    }
}

impl Mp4Box for TkhdBox {
    const TYPE: BoxType = BoxType::TkhdBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "creation_time={} track_id={} duration={} layer={} volume={} matrix={} width={} height={}",
            self.creation_time,
            self.track_id,
            self.duration,
            self.layer,
            self.volume.value(),
            self.matrix,
            self.width.value(),
            self.height.value()
        );
        Ok(s)
    }
}

impl BlockReader for TkhdBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let (creation_time, modification_time, track_id, _, duration) = if version == 1 {
            (
                reader.get_u64(),
                reader.get_u64(),
                reader.get_u32(),
                reader.get_u32(),
                reader.get_u64(),
            )
        } else if version == 0 {
            (
                reader.get_u32() as u64,
                reader.get_u32() as u64,
                reader.get_u32(),
                reader.get_u32(),
                reader.get_u32() as u64,
            )
        } else {
            return Err(BoxError::InvalidData("version must be 0 or 1"));
        };

        reader.get_u64(); // reserved

        let layer = reader.get_u16();
        let alternate_group = reader.get_u16();
        let volume = FixedPointU8::new_raw(reader.get_u16());

        reader.get_u16(); // reserved

        let matrix = Matrix {
            a: reader.get_i32(),
            b: reader.get_i32(),
            u: reader.get_i32(),
            c: reader.get_i32(),
            d: reader.get_i32(),
            v: reader.get_i32(),
            x: reader.get_i32(),
            y: reader.get_i32(),
            w: reader.get_i32(),
        };

        let width = FixedPointU16::new_raw(reader.get_u32());
        let height = FixedPointU16::new_raw(reader.get_u32());

        Ok(TkhdBox {
            version,
            flags,
            creation_time,
            modification_time,
            track_id,
            duration,
            layer,
            alternate_group,
            volume,
            matrix,
            width,
            height,
        })
    }

    fn size_hint() -> usize {
        84
    }
}

impl<W: Write> WriteBox<&mut W> for TkhdBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        if self.version == 1 {
            writer.write_u64::<BigEndian>(self.creation_time)?;
            writer.write_u64::<BigEndian>(self.modification_time)?;
            writer.write_u32::<BigEndian>(self.track_id)?;
            writer.write_u32::<BigEndian>(0)?; // reserved
            writer.write_u64::<BigEndian>(self.duration)?;
        } else if self.version == 0 {
            writer.write_u32::<BigEndian>(self.creation_time as u32)?;
            writer.write_u32::<BigEndian>(self.modification_time as u32)?;
            writer.write_u32::<BigEndian>(self.track_id)?;
            writer.write_u32::<BigEndian>(0)?; // reserved
            writer.write_u32::<BigEndian>(self.duration as u32)?;
        } else {
            return Err(BoxError::InvalidData("version must be 0 or 1"));
        }

        writer.write_u64::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.layer)?;
        writer.write_u16::<BigEndian>(self.alternate_group)?;
        writer.write_u16::<BigEndian>(self.volume.raw_value())?;

        writer.write_u16::<BigEndian>(0)?; // reserved

        writer.write_i32::<BigEndian>(self.matrix.a)?;
        writer.write_i32::<BigEndian>(self.matrix.b)?;
        writer.write_i32::<BigEndian>(self.matrix.u)?;
        writer.write_i32::<BigEndian>(self.matrix.c)?;
        writer.write_i32::<BigEndian>(self.matrix.d)?;
        writer.write_i32::<BigEndian>(self.matrix.v)?;
        writer.write_i32::<BigEndian>(self.matrix.x)?;
        writer.write_i32::<BigEndian>(self.matrix.y)?;
        writer.write_i32::<BigEndian>(self.matrix.w)?;

        writer.write_u32::<BigEndian>(self.width.raw_value())?;
        writer.write_u32::<BigEndian>(self.height.raw_value())?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[tokio::test]
    async fn test_tkhd32() {
        let src_box = TkhdBox {
            version: 0,
            flags: TrackFlag::TrackEnabled as u32,
            creation_time: 100,
            modification_time: 200,
            track_id: 1,
            duration: 634634,
            layer: 0,
            alternate_group: 0,
            volume: FixedPointU8::new(1),
            matrix: Matrix::default(),
            width: FixedPointU16::new(512),
            height: FixedPointU16::new(288),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read(&mut reader, &mut 0).await.unwrap().unwrap();
        assert_eq!(header.kind, BoxType::TkhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = TkhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[tokio::test]
    async fn test_tkhd64() {
        let src_box = TkhdBox {
            version: 1,
            flags: TrackFlag::TrackEnabled as u32,
            creation_time: 100,
            modification_time: 200,
            track_id: 1,
            duration: 634634,
            layer: 0,
            alternate_group: 0,
            volume: FixedPointU8::new(1),
            matrix: Matrix::default(),
            width: FixedPointU16::new(512),
            height: FixedPointU16::new(288),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read(&mut reader, &mut 0).await.unwrap().unwrap();
        assert_eq!(header.kind, BoxType::TkhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = TkhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MvhdBox {
    pub version: u8,
    pub flags: u32,
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,

    #[serde(with = "value_u32")]
    pub rate: FixedPointU16,
    #[serde(with = "value_u8")]
    pub volume: FixedPointU8,

    pub matrix: tkhd::Matrix,

    pub next_track_id: u32,
}

impl MvhdBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MvhdBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;
        if self.version == 1 {
            size += 28;
        } else if self.version == 0 {
            size += 16;
        }
        size += 80;
        size
    }
}

impl Default for MvhdBox {
    fn default() -> Self {
        MvhdBox {
            version: 0,
            flags: 0,
            creation_time: 0,
            modification_time: 0,
            timescale: 1000,
            duration: 0,
            rate: FixedPointU16::new(1),
            matrix: tkhd::Matrix::default(),
            volume: FixedPointU8::new(1),
            next_track_id: 1,
        }
    }
}

impl Mp4Box for MvhdBox {
    const TYPE: BoxType = BoxType::MvhdBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "creation_time={} timescale={} duration={} rate={} volume={}, matrix={}, next_track_id={}",
            self.creation_time,
            self.timescale,
            self.duration,
            self.rate.value(),
            self.volume.value(),
            self.matrix,
            self.next_track_id
        );
        Ok(s)
    }
}

impl BlockReader for MvhdBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let (creation_time, modification_time, timescale, duration) = if version == 1 {
            if reader.remaining() < Self::size_hint() - 4 + 12 {
                return Err(Error::InvalidData("expected more bytes"));
            }

            (
                reader.get_u64(),
                reader.get_u64(),
                reader.get_u32(),
                reader.get_u64(),
            )
        } else if version == 0 {
            (
                reader.get_u32() as u64,
                reader.get_u32() as u64,
                reader.get_u32(),
                reader.get_u32() as u64,
            )
        } else {
            return Err(Error::InvalidData("version must be 0 or 1"));
        };

        let rate = FixedPointU16::new_raw(reader.get_u32());
        let volume = FixedPointU8::new_raw(reader.get_u16());

        reader.get_u16(); // reserved = 0
        reader.get_u64(); // reserved = 0

        let matrix = tkhd::Matrix {
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

        reader.skip(24);

        let next_track_id = reader.get_u32();

        Ok(MvhdBox {
            version,
            flags,
            creation_time,
            modification_time,
            timescale,
            duration,
            rate,
            volume,
            matrix,
            next_track_id,
        })
    }

    fn size_hint() -> usize {
        100
    }
}

impl<W: Write> WriteBox<&mut W> for MvhdBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        if self.version == 1 {
            writer.write_u64::<BigEndian>(self.creation_time)?;
            writer.write_u64::<BigEndian>(self.modification_time)?;
            writer.write_u32::<BigEndian>(self.timescale)?;
            writer.write_u64::<BigEndian>(self.duration)?;
        } else if self.version == 0 {
            writer.write_u32::<BigEndian>(self.creation_time as u32)?;
            writer.write_u32::<BigEndian>(self.modification_time as u32)?;
            writer.write_u32::<BigEndian>(self.timescale)?;
            writer.write_u32::<BigEndian>(self.duration as u32)?;
        } else {
            return Err(Error::InvalidData("version must be 0 or 1"));
        }
        writer.write_u32::<BigEndian>(self.rate.raw_value())?;

        writer.write_u16::<BigEndian>(self.volume.raw_value())?;

        writer.write_u16::<BigEndian>(0)?; // reserved = 0

        writer.write_u64::<BigEndian>(0)?; // reserved = 0

        writer.write_i32::<BigEndian>(self.matrix.a)?;
        writer.write_i32::<BigEndian>(self.matrix.b)?;
        writer.write_i32::<BigEndian>(self.matrix.u)?;
        writer.write_i32::<BigEndian>(self.matrix.c)?;
        writer.write_i32::<BigEndian>(self.matrix.d)?;
        writer.write_i32::<BigEndian>(self.matrix.v)?;
        writer.write_i32::<BigEndian>(self.matrix.x)?;
        writer.write_i32::<BigEndian>(self.matrix.y)?;
        writer.write_i32::<BigEndian>(self.matrix.w)?;

        write_zeros(writer, 24)?; // pre_defined = 0

        writer.write_u32::<BigEndian>(self.next_track_id)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_mvhd32() {
        let src_box = MvhdBox {
            version: 0,
            flags: 0,
            creation_time: 100,
            modification_time: 200,
            timescale: 1000,
            duration: 634634,
            rate: FixedPointU16::new(1),
            volume: FixedPointU8::new(1),
            matrix: tkhd::Matrix::default(),
            next_track_id: 1,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MvhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = MvhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_mvhd64() {
        let src_box = MvhdBox {
            version: 1,
            flags: 0,
            creation_time: 100,
            modification_time: 200,
            timescale: 1000,
            duration: 634634,
            rate: FixedPointU16::new(1),
            volume: FixedPointU8::new(1),
            matrix: tkhd::Matrix::default(),
            next_track_id: 1,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MvhdBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = MvhdBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

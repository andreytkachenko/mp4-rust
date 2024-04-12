use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Avc1Box {
    pub data_reference_index: u16,
    pub width: u16,
    pub height: u16,

    #[serde(with = "value_u32")]
    pub horizresolution: FixedPointU16,

    #[serde(with = "value_u32")]
    pub vertresolution: FixedPointU16,
    pub frame_count: u16,
    pub depth: u16,
    pub avcc: AvcCBox,
}

impl Default for Avc1Box {
    fn default() -> Self {
        Avc1Box {
            data_reference_index: 0,
            width: 0,
            height: 0,
            horizresolution: FixedPointU16::new(0x48),
            vertresolution: FixedPointU16::new(0x48),
            frame_count: 1,
            depth: 0x0018,
            avcc: AvcCBox::default(),
        }
    }
}

impl Avc1Box {
    pub fn new(config: &AvcConfig) -> Self {
        Avc1Box {
            data_reference_index: 1,
            width: config.width,
            height: config.height,
            horizresolution: FixedPointU16::new(0x48),
            vertresolution: FixedPointU16::new(0x48),
            frame_count: 1,
            depth: 0x0018,
            avcc: AvcCBox::new(&config.seq_param_set, &config.pic_param_set),
        }
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + 8 + 70 + self.avcc.box_size()
    }

    fn box_type(&self) -> BoxType {
        BoxType::Avc1Box
    }
}

impl Mp4Box for Avc1Box {
    const TYPE: BoxType = BoxType::Avc1Box;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "data_reference_index={} width={} height={} frame_count={}",
            self.data_reference_index, self.width, self.height, self.frame_count
        );
        Ok(s)
    }
}

impl BlockReader for Avc1Box {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        reader.get_u32(); // reserved
        reader.get_u16(); // reserved

        let data_reference_index = reader.get_u16();

        reader.get_u32(); // pre-defined, reserved
        reader.get_u64(); // pre-defined
        reader.get_u32(); // pre-defined

        let width = reader.get_u16();
        let height = reader.get_u16();

        let horizresolution = FixedPointU16::new_raw(reader.get_u32());
        let vertresolution = FixedPointU16::new_raw(reader.get_u32());

        reader.get_u32(); // reserved

        let frame_count = reader.get_u16();

        reader.skip(32); // compressorname

        let depth = reader.get_u16();

        reader.get_i16(); // pre-defined

        Ok(Avc1Box {
            data_reference_index,
            width,
            height,
            horizresolution,
            vertresolution,
            frame_count,
            depth,
            avcc: reader.find_box::<AvcCBox>()?,
        })
    }

    fn size_hint() -> usize {
        78
    }
}

impl<W: Write> WriteBox<&mut W> for Avc1Box {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_u32::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.data_reference_index)?;

        writer.write_u32::<BigEndian>(0)?; // pre-defined, reserved
        writer.write_u64::<BigEndian>(0)?; // pre-defined
        writer.write_u32::<BigEndian>(0)?; // pre-defined
        writer.write_u16::<BigEndian>(self.width)?;
        writer.write_u16::<BigEndian>(self.height)?;
        writer.write_u32::<BigEndian>(self.horizresolution.raw_value())?;
        writer.write_u32::<BigEndian>(self.vertresolution.raw_value())?;
        writer.write_u32::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.frame_count)?;
        // skip compressorname
        write_zeros(writer, 32)?;
        writer.write_u16::<BigEndian>(self.depth)?;
        writer.write_i16::<BigEndian>(-1)?; // pre-defined

        self.avcc.write_box(writer)?;

        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct AvcCBox {
    pub configuration_version: u8,
    pub avc_profile_indication: u8,
    pub profile_compatibility: u8,
    pub avc_level_indication: u8,
    pub length_size_minus_one: u8,
    pub sequence_parameter_sets: Vec<NalUnit>,
    pub picture_parameter_sets: Vec<NalUnit>,
}

impl AvcCBox {
    pub fn new(sps: &[u8], pps: &[u8]) -> Self {
        Self {
            configuration_version: 1,
            avc_profile_indication: sps[1],
            profile_compatibility: sps[2],
            avc_level_indication: sps[3],
            length_size_minus_one: 0xff, // length_size = 4
            sequence_parameter_sets: vec![NalUnit::from(sps)],
            picture_parameter_sets: vec![NalUnit::from(pps)],
        }
    }
}

impl Mp4Box for AvcCBox {
    const TYPE: BoxType = BoxType::AvcCBox;

    fn box_size(&self) -> u64 {
        let mut size = HEADER_SIZE + 7;
        for sps in self.sequence_parameter_sets.iter() {
            size += sps.size() as u64;
        }
        for pps in self.picture_parameter_sets.iter() {
            size += pps.size() as u64;
        }
        size
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("avc_profile_indication={}", self.avc_profile_indication);
        Ok(s)
    }
}

impl BlockReader for AvcCBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let configuration_version = reader.get_u8();
        let avc_profile_indication = reader.get_u8();
        let profile_compatibility = reader.get_u8();
        let avc_level_indication = reader.get_u8();
        let length_size_minus_one = reader.get_u8() & 0x3;
        let num_of_spss = reader.get_u8() & 0x1F;

        let mut sequence_parameter_sets = Vec::with_capacity(num_of_spss as usize);
        for _ in 0..num_of_spss {
            let nal_unit = NalUnit::read(reader)?;
            sequence_parameter_sets.push(nal_unit);
        }

        let num_of_ppss = reader.get_u8();

        let mut picture_parameter_sets = Vec::with_capacity(num_of_ppss as usize);
        for _ in 0..num_of_ppss {
            let nal_unit = NalUnit::read(reader)?;
            picture_parameter_sets.push(nal_unit);
        }

        Ok(AvcCBox {
            configuration_version,
            avc_profile_indication,
            profile_compatibility,
            avc_level_indication,
            length_size_minus_one,
            sequence_parameter_sets,
            picture_parameter_sets,
        })
    }

    fn size_hint() -> usize {
        7
    }
}

impl<W: Write> WriteBox<&mut W> for AvcCBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        writer.write_u8(self.configuration_version)?;
        writer.write_u8(self.avc_profile_indication)?;
        writer.write_u8(self.profile_compatibility)?;
        writer.write_u8(self.avc_level_indication)?;
        writer.write_u8(self.length_size_minus_one | 0xFC)?;
        writer.write_u8(self.sequence_parameter_sets.len() as u8 | 0xE0)?;
        for sps in self.sequence_parameter_sets.iter() {
            sps.write(writer)?;
        }
        writer.write_u8(self.picture_parameter_sets.len() as u8)?;
        for pps in self.picture_parameter_sets.iter() {
            pps.write(writer)?;
        }
        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct NalUnit {
    pub bytes: Vec<u8>,
}

impl From<&[u8]> for NalUnit {
    fn from(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }
}

impl NalUnit {
    fn size(&self) -> usize {
        2 + self.bytes.len()
    }

    fn read<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let length = reader.try_get_u16()? as usize;

        Ok(NalUnit {
            bytes: reader.collect(length)?,
        })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<u64> {
        writer.write_u16::<BigEndian>(self.bytes.len() as u16)?;
        writer.write_all(&self.bytes)?;
        Ok(self.size() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_avc1() {
        let src_box = Avc1Box {
            data_reference_index: 1,
            width: 320,
            height: 240,
            horizresolution: FixedPointU16::new(0x48),
            vertresolution: FixedPointU16::new(0x48),
            frame_count: 1,
            depth: 24,
            avcc: AvcCBox {
                configuration_version: 1,
                avc_profile_indication: 100,
                profile_compatibility: 0,
                avc_level_indication: 13,
                length_size_minus_one: 3,
                sequence_parameter_sets: vec![NalUnit {
                    bytes: vec![
                        0x67, 0x64, 0x00, 0x0D, 0xAC, 0xD9, 0x41, 0x41, 0xFA, 0x10, 0x00, 0x00,
                        0x03, 0x00, 0x10, 0x00, 0x00, 0x03, 0x03, 0x20, 0xF1, 0x42, 0x99, 0x60,
                    ],
                }],
                picture_parameter_sets: vec![NalUnit {
                    bytes: vec![0x68, 0xEB, 0xE3, 0xCB, 0x22, 0xC0],
                }],
            },
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let header = BoxHeader::read_sync(&mut buf.as_slice()).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::Avc1Box);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = Avc1Box::read_block(&mut buf.as_slice()).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hev1Box {
    pub data_reference_index: u16,
    pub width: u16,
    pub height: u16,

    #[serde(with = "value_u32")]
    pub horizresolution: FixedPointU16,

    #[serde(with = "value_u32")]
    pub vertresolution: FixedPointU16,
    pub frame_count: u16,
    pub depth: u16,
    pub hvcc: HvcCBox,
}

impl Default for Hev1Box {
    fn default() -> Self {
        Hev1Box {
            data_reference_index: 0,
            width: 0,
            height: 0,
            horizresolution: FixedPointU16::new(0x48),
            vertresolution: FixedPointU16::new(0x48),
            frame_count: 1,
            depth: 0x0018,
            hvcc: HvcCBox::default(),
        }
    }
}

impl Hev1Box {
    pub fn new(config: &HevcConfig) -> Self {
        Hev1Box {
            data_reference_index: 1,
            width: config.width,
            height: config.height,
            horizresolution: FixedPointU16::new(0x48),
            vertresolution: FixedPointU16::new(0x48),
            frame_count: 1,
            depth: 0x0018,
            hvcc: HvcCBox::new(),
        }
    }

    pub fn get_type(&self) -> BoxType {
        BoxType::Hev1Box
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + 8 + 70 + self.hvcc.box_size()
    }
}

impl Mp4Box for Hev1Box {
    const TYPE: BoxType = BoxType::Hev1Box;

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

impl BlockReader for Hev1Box {
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

        Ok(Hev1Box {
            data_reference_index,
            width,
            height,
            horizresolution,
            vertresolution,
            frame_count,
            depth,
            hvcc: reader.find_box::<HvcCBox>()?,
        })
    }

    fn size_hint() -> usize {
        78
    }
}

impl<W: Write> WriteBox<&mut W> for Hev1Box {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

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

        self.hvcc.write_box(writer)?;

        Ok(size)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HvcCBox {
    pub configuration_version: u8,
    pub general_profile_space: u8,
    pub general_tier_flag: bool,
    pub general_profile_idc: u8,
    pub general_profile_compatibility_flags: u32,
    pub general_constraint_indicator_flag: u64,
    pub general_level_idc: u8,
    pub min_spatial_segmentation_idc: u16,
    pub parallelism_type: u8,
    pub chroma_format_idc: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub avg_frame_rate: u16,
    pub constant_frame_rate: u8,
    pub num_temporal_layers: u8,
    pub temporal_id_nested: bool,
    pub length_size_minus_one: u8,
    pub arrays: Vec<HvcCArray>,
}

impl HvcCBox {
    pub fn new() -> Self {
        Self {
            configuration_version: 1,
            ..Default::default()
        }
    }
}

impl Mp4Box for HvcCBox {
    const TYPE: BoxType = BoxType::HvcCBox;

    fn box_size(&self) -> u64 {
        HEADER_SIZE
            + 23
            + self
                .arrays
                .iter()
                .map(|a| 3 + a.nalus.iter().map(|x| 2 + x.data.len() as u64).sum::<u64>())
                .sum::<u64>()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        Ok(format!("configuration_version={} general_profile_space={} general_tier_flag={} general_profile_idc={} general_profile_compatibility_flags={} general_constraint_indicator_flag={} general_level_idc={} min_spatial_segmentation_idc={} parallelism_type={} chroma_format_idc={} bit_depth_luma_minus8={} bit_depth_chroma_minus8={} avg_frame_rate={} constant_frame_rate={} num_temporal_layers={} temporal_id_nested={} length_size_minus_one={}", 
            self.configuration_version,
            self.general_profile_space,
            self.general_tier_flag,
            self.general_profile_idc,
            self.general_profile_compatibility_flags,
            self.general_constraint_indicator_flag,
            self.general_level_idc,
            self.min_spatial_segmentation_idc,
            self.parallelism_type,
            self.chroma_format_idc,
            self.bit_depth_luma_minus8,
            self.bit_depth_chroma_minus8,
            self.avg_frame_rate,
            self.constant_frame_rate,
            self.num_temporal_layers,
            self.temporal_id_nested,
            self.length_size_minus_one
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct HvcCArrayNalu {
    pub size: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct HvcCArray {
    pub completeness: bool,
    pub nal_unit_type: u8,
    pub nalus: Vec<HvcCArrayNalu>,
}

impl BlockReader for HvcCBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let configuration_version = reader.get_u8();
        let params = reader.get_u8();
        let general_profile_space = params & 0b11000000 >> 6;
        let general_tier_flag = (params & 0b00100000 >> 5) > 0;
        let general_profile_idc = params & 0b00011111;

        let general_profile_compatibility_flags = reader.get_u32();
        let general_constraint_indicator_flag = reader.get_u48();

        let general_level_idc = reader.get_u8();
        let min_spatial_segmentation_idc = reader.get_u16() & 0x0FFF;
        let parallelism_type = reader.get_u8() & 0b11;
        let chroma_format_idc = reader.get_u8() & 0b11;
        let bit_depth_luma_minus8 = reader.get_u8() & 0b111;
        let bit_depth_chroma_minus8 = reader.get_u8() & 0b111;
        let avg_frame_rate = reader.get_u16();

        let params = reader.get_u8();
        let constant_frame_rate = params & 0b11000000 >> 6;
        let num_temporal_layers = params & 0b00111000 >> 3;
        let temporal_id_nested = (params & 0b00000100 >> 2) > 0;
        let length_size_minus_one = params & 0b000011;

        let num_of_arrays = reader.get_u8();

        if reader.remaining() < num_of_arrays as usize * 3 {
            return Err(Error::InvalidData(""));
        }

        let mut arrays = Vec::with_capacity(num_of_arrays as _);

        for _ in 0..num_of_arrays {
            let params = reader.get_u8();
            let num_nalus = reader.get_u16();

            if reader.remaining() < num_nalus as usize * 2 {
                return Err(Error::InvalidData(""));
            }

            let mut nalus = Vec::with_capacity(num_nalus as usize);

            for _ in 0..num_nalus {
                let size = reader.get_u16();

                nalus.push(HvcCArrayNalu {
                    size,
                    data: reader.collect(size as _)?,
                })
            }

            arrays.push(HvcCArray {
                completeness: (params & 0b10000000) > 0,
                nal_unit_type: params & 0b111111,
                nalus,
            });
        }

        Ok(HvcCBox {
            configuration_version,
            general_profile_space,
            general_tier_flag,
            general_profile_idc,
            general_profile_compatibility_flags,
            general_constraint_indicator_flag,
            general_level_idc,
            min_spatial_segmentation_idc,
            parallelism_type,
            chroma_format_idc,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            avg_frame_rate,
            constant_frame_rate,
            num_temporal_layers,
            temporal_id_nested,
            length_size_minus_one,
            arrays,
        })
    }

    fn size_hint() -> usize {
        23
    }
}

impl<W: Write> WriteBox<&mut W> for HvcCBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        writer.write_u8(self.configuration_version)?;
        let general_profile_space = (self.general_profile_space & 0b11) << 6;
        let general_tier_flag = u8::from(self.general_tier_flag) << 5;
        let general_profile_idc = self.general_profile_idc & 0b11111;

        writer.write_u8(general_profile_space | general_tier_flag | general_profile_idc)?;
        writer.write_u32::<BigEndian>(self.general_profile_compatibility_flags)?;
        writer.write_u48::<BigEndian>(self.general_constraint_indicator_flag)?;
        writer.write_u8(self.general_level_idc)?;

        writer.write_u16::<BigEndian>(self.min_spatial_segmentation_idc & 0x0FFF)?;
        writer.write_u8(self.parallelism_type & 0b11)?;
        writer.write_u8(self.chroma_format_idc & 0b11)?;
        writer.write_u8(self.bit_depth_luma_minus8 & 0b111)?;
        writer.write_u8(self.bit_depth_chroma_minus8 & 0b111)?;
        writer.write_u16::<BigEndian>(self.avg_frame_rate)?;

        let constant_frame_rate = (self.constant_frame_rate & 0b11) << 6;
        let num_temporal_layers = (self.num_temporal_layers & 0b111) << 3;
        let temporal_id_nested = u8::from(self.temporal_id_nested) << 2;
        let length_size_minus_one = self.length_size_minus_one & 0b11;
        writer.write_u8(
            constant_frame_rate | num_temporal_layers | temporal_id_nested | length_size_minus_one,
        )?;
        writer.write_u8(self.arrays.len() as u8)?;
        for arr in &self.arrays {
            writer.write_u8((arr.nal_unit_type & 0b111111) | u8::from(arr.completeness) << 7)?;
            writer.write_u16::<BigEndian>(arr.nalus.len() as _)?;

            for nalu in &arr.nalus {
                writer.write_u16::<BigEndian>(nalu.size)?;
                writer.write_all(&nalu.data)?;
            }
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_hev1() {
        let src_box = Hev1Box {
            data_reference_index: 1,
            width: 320,
            height: 240,
            horizresolution: FixedPointU16::new(0x48),
            vertresolution: FixedPointU16::new(0x48),
            frame_count: 1,
            depth: 24,
            hvcc: HvcCBox {
                configuration_version: 1,
                ..Default::default()
            },
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::Hev1Box);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = Hev1Box::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

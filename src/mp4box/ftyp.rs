use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct FtypBox {
    pub major_brand: FourCC,
    pub minor_version: u32,
    pub compatible_brands: Vec<FourCC>,
}

impl FtypBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::FtypBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + 8 + (4 * self.compatible_brands.len() as u64)
    }
}

impl Mp4Box for FtypBox {
    const TYPE: BoxType = BoxType::FtypBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let mut compatible_brands = Vec::new();
        for brand in self.compatible_brands.iter() {
            compatible_brands.push(brand.to_string());
        }
        let s = format!(
            "major_brand={} minor_version={} compatible_brands={}",
            self.major_brand,
            self.minor_version,
            compatible_brands.join("-")
        );
        Ok(s)
    }
}

impl BlockReader for FtypBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let brand_count = (reader.remaining() - 16) / 4; // header + major + minor

        let major = reader.get_u32();
        let minor = reader.get_u32();

        let mut brands = Vec::new();
        for _ in 0..brand_count {
            let b = reader.get_u32();
            brands.push(From::from(b));
        }

        Ok(FtypBox {
            major_brand: From::from(major),
            minor_version: minor,
            compatible_brands: brands,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for FtypBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        writer.write_u32::<BigEndian>((&self.major_brand).into())?;
        writer.write_u32::<BigEndian>(self.minor_version)?;
        for b in self.compatible_brands.iter() {
            writer.write_u32::<BigEndian>(b.into())?;
        }
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_ftyp() {
        let src_box = FtypBox {
            major_brand: str::parse("isom").unwrap(),
            minor_version: 0,
            compatible_brands: vec![
                str::parse("isom").unwrap(),
                str::parse("iso2").unwrap(),
                str::parse("avc1").unwrap(),
                str::parse("mp41").unwrap(),
            ],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::FtypBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = FtypBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

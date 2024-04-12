use serde::Serialize;
use std::convert::TryFrom;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct DataBox {
    pub data: Vec<u8>,
    pub data_type: DataType,
}

impl DataBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::DataBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        size += 4; // data_type
        size += 4; // reserved
        size += self.data.len() as u64;
        size
    }
}

impl Mp4Box for DataBox {
    const TYPE: BoxType = BoxType::DataBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("type={:?} len={}", self.data_type, self.data.len());
        Ok(s)
    }
}

impl BlockReader for DataBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let data_type = DataType::try_from(reader.get_u32())?;
        reader.get_u32(); // reserved = 0

        Ok(DataBox {
            data: reader.collect(reader.remaining())?,
            data_type,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for DataBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        writer.write_u32::<BigEndian>(self.data_type.clone() as u32)?;
        writer.write_u32::<BigEndian>(0)?; // reserved = 0
        writer.write_all(&self.data)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_data() {
        let src_box = DataBox {
            data_type: DataType::Text,
            data: b"test_data".to_vec(),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::DataBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = DataBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_data_empty() {
        let src_box = DataBox::default();
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::DataBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = DataBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

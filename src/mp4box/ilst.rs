use std::borrow::Cow;
use std::collections::HashMap;

use byteorder::ByteOrder;
use serde::Serialize;

use crate::mp4box::data::DataBox;
use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct IlstBox {
    pub items: HashMap<MetadataKey, DataBox>,
}

impl IlstBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::IlstBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        for item in self.items.values() {
            size += item.get_size();
        }
        size
    }
}

impl Mp4Box for IlstBox {
    const TYPE: BoxType = BoxType::IlstBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("item_count={}", self.items.len());
        Ok(s)
    }
}

impl BlockReader for IlstBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let mut items = HashMap::new();

        while let Some(mut bx) = reader.get_box()? {
            match bx.kind {
                BoxType::NameBox => {
                    if let Some(title) = bx.inner.try_find_box::<DataBox>()? {
                        items.insert(MetadataKey::Title, title);
                    }
                }

                BoxType::DayBox => {
                    if let Some(day) = bx.inner.try_find_box::<DataBox>()? {
                        items.insert(MetadataKey::Year, day);
                    }
                }

                BoxType::CovrBox => {
                    if let Some(cover) = bx.inner.try_find_box::<DataBox>()? {
                        items.insert(MetadataKey::Poster, cover);
                    }
                }

                BoxType::DescBox => {
                    if let Some(summary) = bx.inner.try_find_box::<DataBox>()? {
                        items.insert(MetadataKey::Summary, summary);
                    }
                }

                _ => continue,
            }
        }

        Ok(IlstBox { items })
    }

    fn size_hint() -> usize {
        0
    }
}

impl<W: Write> WriteBox<&mut W> for IlstBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        for (key, value) in &self.items {
            let name = match key {
                MetadataKey::Title => BoxType::NameBox,
                MetadataKey::Year => BoxType::DayBox,
                MetadataKey::Poster => BoxType::CovrBox,
                MetadataKey::Summary => BoxType::DescBox,
            };

            BoxHeader::new(name, value.get_size()).write(writer)?;
            value.write_box(writer)?;
        }
        Ok(size)
    }
}

impl<'a> Metadata<'a> for IlstBox {
    fn title(&self) -> Option<Cow<str>> {
        self.items.get(&MetadataKey::Title).map(item_to_str)
    }

    fn year(&self) -> Option<u32> {
        self.items.get(&MetadataKey::Year).and_then(item_to_u32)
    }

    fn poster(&self) -> Option<&[u8]> {
        self.items.get(&MetadataKey::Poster).map(item_to_bytes)
    }

    fn summary(&self) -> Option<Cow<str>> {
        self.items.get(&MetadataKey::Summary).map(item_to_str)
    }
}

fn item_to_bytes(item: &DataBox) -> &[u8] {
    &item.data
}

fn item_to_str(item: &DataBox) -> Cow<str> {
    String::from_utf8_lossy(&item.data)
}

fn item_to_u32(item: &DataBox) -> Option<u32> {
    match item.data_type {
        DataType::Binary if item.data.len() == 4 => Some(BigEndian::read_u32(&item.data)),
        DataType::Text => String::from_utf8_lossy(&item.data).parse::<u32>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_ilst() {
        let src_year = DataBox {
            data_type: DataType::Text,
            data: b"test_year".to_vec(),
        };

        let src_box = IlstBox {
            items: [
                (MetadataKey::Title, DataBox::default()),
                (MetadataKey::Year, src_year),
                (MetadataKey::Poster, DataBox::default()),
                (MetadataKey::Summary, DataBox::default()),
            ]
            .into(),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::IlstBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = IlstBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_ilst_empty() {
        let src_box = IlstBox::default();
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::IlstBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = IlstBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

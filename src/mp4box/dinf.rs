use serde::Serialize;
use std::io::Write;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct DinfBox {
    dref: DrefBox,
}

impl DinfBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::DinfBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + self.dref.box_size()
    }
}

impl Mp4Box for DinfBox {
    const TYPE: BoxType = BoxType::DinfBox;

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

impl BlockReader for DinfBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        Ok(DinfBox {
            dref: reader.find_box::<DrefBox>()?,
        })
    }

    fn size_hint() -> usize {
        DrefBox::size_hint()
    }
}

impl<W: Write> WriteBox<&mut W> for DinfBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;
        self.dref.write_box(writer)?;
        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DrefBox {
    pub version: u8,
    pub flags: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<UrlBox>,
}

impl Default for DrefBox {
    fn default() -> Self {
        DrefBox {
            version: 0,
            flags: 0,
            url: Some(UrlBox::default()),
        }
    }
}

impl DrefBox {
    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE + 4;
        if let Some(ref url) = self.url {
            size += url.box_size();
        }
        size
    }
}

impl Mp4Box for DrefBox {
    const TYPE: BoxType = BoxType::DrefBox;

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

impl BlockReader for DrefBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);
        let mut url = None;
        let entry_count = reader.get_u32();

        for _i in 0..entry_count {
            url = reader.try_find_box()?;
        }

        Ok(DrefBox {
            version,
            flags,
            url,
        })
    }

    fn size_hint() -> usize {
        8
    }
}

impl<W: Write> WriteBox<&mut W> for DrefBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        writer.write_u32::<BigEndian>(1)?;

        if let Some(ref url) = self.url {
            url.write_box(writer)?;
        }

        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UrlBox {
    pub version: u8,
    pub flags: u32,
    pub location: String,
}

impl Default for UrlBox {
    fn default() -> Self {
        UrlBox {
            version: 0,
            flags: 1,
            location: String::default(),
        }
    }
}

impl UrlBox {
    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;

        if !self.location.is_empty() {
            size += self.location.bytes().len() as u64 + 1;
        }

        size
    }
}

impl Mp4Box for UrlBox {
    const TYPE: BoxType = BoxType::UrlBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("location={}", self.location);
        Ok(s)
    }
}

impl BlockReader for UrlBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        Ok(UrlBox {
            version,
            flags,
            location: reader.get_null_terminated_string(),
        })
    }

    fn size_hint() -> usize {
        4
    }
}

impl<W: Write> WriteBox<&mut W> for UrlBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;

        if !self.location.is_empty() {
            writer.write_all(self.location.as_bytes())?;
            writer.write_u8(0)?;
        }

        Ok(size)
    }
}

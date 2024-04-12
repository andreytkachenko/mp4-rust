use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};
use serde::Serialize;

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct EmsgBox {
    pub version: u8,
    pub flags: u32,
    pub timescale: u32,
    pub presentation_time: Option<u64>,
    pub presentation_time_delta: Option<u32>,
    pub event_duration: u32,
    pub id: u32,
    pub scheme_id_uri: String,
    pub value: String,
    pub message_data: Vec<u8>,
}

impl EmsgBox {
    fn size_without_message(version: u8, scheme_id_uri: &str, value: &str) -> u64 {
        HEADER_SIZE + HEADER_EXT_SIZE +
            4 + // id
            Self::time_size(version) +
            (scheme_id_uri.len() + 1) as u64 +
            (value.len() as u64 + 1)
    }

    fn time_size(version: u8) -> u64 {
        match version {
            0 => 12,
            1 => 16,
            _ => panic!("version must be 0 or 1"),
        }
    }
}

impl Mp4Box for EmsgBox {
    const TYPE: BoxType = BoxType::EmsgBox;

    fn box_size(&self) -> u64 {
        Self::size_without_message(self.version, &self.scheme_id_uri, &self.value)
            + self.message_data.len() as u64
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("id={} value={}", self.id, self.value);
        Ok(s)
    }
}

impl BlockReader for EmsgBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (version, flags) = read_box_header_ext(reader);

        let (
            timescale,
            presentation_time,
            presentation_time_delta,
            event_duration,
            id,
            scheme_id_uri,
            value,
        ) = match version {
            0 => {
                let scheme_id_uri = reader.get_null_terminated_string();
                let value = reader.get_null_terminated_string();

                (
                    reader.get_u32(),
                    None,
                    Some(reader.get_u32()),
                    reader.get_u32(),
                    reader.get_u32(),
                    scheme_id_uri,
                    value,
                )
            }
            1 => (
                reader.get_u32(),
                Some(reader.get_u64()),
                None,
                reader.get_u32(),
                reader.get_u32(),
                reader.get_null_terminated_string(),
                reader.get_null_terminated_string(),
            ),
            _ => return Err(Error::InvalidData("version must be 0 or 1")),
        };

        Ok(EmsgBox {
            version,
            flags,
            timescale,
            presentation_time,
            presentation_time_delta,
            event_duration,
            id,
            scheme_id_uri,
            value,
            message_data: reader.collect(reader.remaining())?,
        })
    }

    fn size_hint() -> usize {
        22
    }
}

impl<W: Write> WriteBox<&mut W> for EmsgBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, self.version, self.flags)?;
        match self.version {
            0 => {
                write_null_terminated_str(writer, &self.scheme_id_uri)?;
                write_null_terminated_str(writer, &self.value)?;
                writer.write_u32::<BigEndian>(self.timescale)?;
                writer.write_u32::<BigEndian>(self.presentation_time_delta.unwrap())?;
                writer.write_u32::<BigEndian>(self.event_duration)?;
                writer.write_u32::<BigEndian>(self.id)?;
            }
            1 => {
                writer.write_u32::<BigEndian>(self.timescale)?;
                writer.write_u64::<BigEndian>(self.presentation_time.unwrap())?;
                writer.write_u32::<BigEndian>(self.event_duration)?;
                writer.write_u32::<BigEndian>(self.id)?;
                write_null_terminated_str(writer, &self.scheme_id_uri)?;
                write_null_terminated_str(writer, &self.value)?;
            }
            _ => return Err(Error::InvalidData("version must be 0 or 1")),
        }

        for &byte in &self.message_data {
            writer.write_u8(byte)?;
        }

        Ok(size)
    }
}

fn write_null_terminated_str<W: Write>(writer: &mut W, string: &str) -> Result<()> {
    for byte in string.bytes() {
        writer.write_u8(byte)?;
    }
    writer.write_u8(0)?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::mp4box::BoxHeader;

    use super::*;

    #[test]
    fn test_emsg_version0() {
        let src_box = EmsgBox {
            version: 0,
            flags: 0,
            timescale: 48000,
            presentation_time: None,
            presentation_time_delta: Some(100),
            event_duration: 200,
            id: 8,
            scheme_id_uri: String::from("foo"),
            value: String::from("foo"),
            message_data: vec![1, 2, 3],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::EmsgBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = EmsgBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }

    #[test]
    fn test_emsg_version1() {
        let src_box = EmsgBox {
            version: 1,
            flags: 0,
            timescale: 48000,
            presentation_time: Some(50000),
            presentation_time_delta: None,
            event_duration: 200,
            id: 8,
            scheme_id_uri: String::from("foo"),
            value: String::from("foo"),
            message_data: vec![3, 2, 1],
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::EmsgBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = EmsgBox::read_block(&mut reader).unwrap();
        assert_eq!(src_box, dst_box);
    }
}

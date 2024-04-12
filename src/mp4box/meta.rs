use serde::Serialize;

use crate::mp4box::hdlr::HdlrBox;
use crate::mp4box::ilst::IlstBox;
use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "hdlr")]
#[serde(rename_all = "lowercase")]
pub enum MetaBox {
    Mdir {
        #[serde(skip_serializing_if = "Option::is_none")]
        ilst: Option<IlstBox>,
    },

    #[serde(skip)]
    Unknown {
        #[serde(skip)]
        hdlr: HdlrBox,

        #[serde(skip)]
        data: Vec<(BoxType, Vec<u8>)>,
    },
}

const MDIR: FourCC = FourCC { value: *b"mdir" };

impl MetaBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MetaBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + HEADER_EXT_SIZE;
        match self {
            Self::Mdir { ilst } => {
                size += HdlrBox::default().box_size();
                if let Some(ilst) = ilst {
                    size += ilst.box_size();
                }
            }
            Self::Unknown { hdlr, data } => {
                size += hdlr.box_size()
                    + data
                        .iter()
                        .map(|(_, data)| data.len() as u64 + HEADER_SIZE)
                        .sum::<u64>()
            }
        }
        size
    }
}

impl Mp4Box for MetaBox {
    const TYPE: BoxType = BoxType::MetaBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = match self {
            Self::Mdir { .. } => "hdlr=ilst".to_string(),
            Self::Unknown { hdlr, data } => {
                format!("hdlr={} data_len={}", hdlr.handler_type, data.len())
            }
        };
        Ok(s)
    }
}

impl Default for MetaBox {
    fn default() -> Self {
        Self::Unknown {
            hdlr: Default::default(),
            data: Default::default(),
        }
    }
}

impl BlockReader for MetaBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let extended_header = reader.get_u32();

        if extended_header != 0 {
            // ISO mp4 requires this header (version & flags) to be 0. Some
            // files skip the extended header and directly start the hdlr box.
            let possible_hdlr = BoxType::from(reader.try_get_u32()?);

            if possible_hdlr == BoxType::HdlrBox {
                // This file skipped the extended header! Go back to start.
                // reader.seek(SeekFrom::Current(-8))?;
            } else {
                // Looks like we actually have a bad version number or flags.
                let v = (extended_header >> 24) as u8;
                return Err(Error::UnsupportedBoxVersion(BoxType::MetaBox, v));
            }
        }

        // find the hdlr box
        let hdlr = reader.find_box::<HdlrBox>()?;

        Ok(match hdlr.handler_type {
            MDIR => MetaBox::Mdir {
                ilst: reader.try_find_box::<IlstBox>()?,
            },
            _ => {
                let mut data = Vec::new();

                while let Some(mut bx) = reader.get_box()? {
                    data.push((bx.kind, bx.inner.collect_remaining()))
                }

                MetaBox::Unknown { hdlr, data }
            }
        })
    }

    fn size_hint() -> usize {
        4
    }
}

impl<W: Write> WriteBox<&mut W> for MetaBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        write_box_header_ext(writer, 0, 0)?;

        let hdlr = match self {
            Self::Mdir { .. } => HdlrBox {
                handler_type: MDIR,
                ..Default::default()
            },
            Self::Unknown { hdlr, .. } => hdlr.clone(),
        };
        hdlr.write_box(writer)?;

        match self {
            Self::Mdir { ilst } => {
                if let Some(ilst) = ilst {
                    ilst.write_box(writer)?;
                }
            }
            Self::Unknown { data, .. } => {
                for (box_type, data) in data {
                    BoxHeader::new(*box_type, data.len() as u64 + HEADER_SIZE).write(writer)?;
                    writer.write_all(data)?;
                }
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
    fn test_meta_mdir_empty() {
        let src_box = MetaBox::Mdir { ilst: None };

        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MetaBox);
        assert_eq!(header.size, src_box.box_size());

        let dst_box = MetaBox::read_block(&mut reader).unwrap();
        assert_eq!(dst_box, src_box);
    }

    #[test]
    fn test_meta_mdir() {
        let src_box = MetaBox::Mdir {
            ilst: Some(IlstBox::default()),
        };

        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MetaBox);
        assert_eq!(header.size, src_box.box_size());

        let dst_box = MetaBox::read_block(&mut reader).unwrap();
        assert_eq!(dst_box, src_box);
    }

    #[test]
    fn test_meta_hdrl_non_first() {
        let data = b"\x00\x00\x00\x7fmeta\x00\x00\x00\x00\x00\x00\x00Qilst\x00\x00\x00I\xa9too\x00\x00\x00Adata\x00\x00\x00\x01\x00\x00\x00\x00TMPGEnc Video Mastering Works 7 Version 7.0.15.17\x00\x00\x00\"hdlr\x00\x00\x00\x00\x00\x00\x00\x00mdirappl\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

        let mut reader = data.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MetaBox);

        let meta_box = MetaBox::read_block(&mut reader).unwrap();

        // this contains \xa9too box in the ilst
        // it designates the tool that created the file, but is not yet supported by this crate
        assert_eq!(
            meta_box,
            MetaBox::Mdir {
                ilst: Some(IlstBox::default())
            }
        );
    }

    #[test]
    fn test_meta_unknown() {
        let src_hdlr = HdlrBox {
            handler_type: FourCC::from(*b"test"),
            ..Default::default()
        };
        let src_data = (BoxType::UnknownBox(0x42494241), b"123".to_vec());
        let src_box = MetaBox::Unknown {
            hdlr: src_hdlr,
            data: vec![src_data],
        };

        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MetaBox);
        assert_eq!(header.size, src_box.box_size());

        let dst_box = MetaBox::read_block(&mut reader).unwrap();
        assert_eq!(dst_box, src_box);
    }
}

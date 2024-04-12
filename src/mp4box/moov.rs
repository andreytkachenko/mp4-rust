use serde::Serialize;
use std::io::Write;

use crate::meta::MetaBox;
use crate::mp4box::*;
use crate::mp4box::{mvex::MvexBox, mvhd::MvhdBox, trak::TrakBox, udta::UdtaBox};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct MoovBox {
    pub mvhd: MvhdBox,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaBox>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mvex: Option<MvexBox>,

    #[serde(rename = "trak")]
    pub traks: Vec<TrakBox>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub udta: Option<UdtaBox>,
}

impl MoovBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MoovBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + self.mvhd.box_size();
        for trak in self.traks.iter() {
            size += trak.box_size();
        }
        if let Some(meta) = &self.meta {
            size += meta.box_size();
        }
        if let Some(udta) = &self.udta {
            size += udta.box_size();
        }
        size
    }
}

impl Mp4Box for MoovBox {
    const TYPE: BoxType = BoxType::MoovBox;

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("traks={}", self.traks.len());
        Ok(s)
    }
}

impl BlockReader for MoovBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        println!("begin reading moov");

        let mut mvhd = None;
        let mut meta = None;
        let mut udta = None;
        let mut mvex = None;
        let mut traks = Vec::new();

        while let Some(mut bx) = reader.get_box()? {
            match bx.kind {
                BoxType::MvhdBox => {
                    mvhd = Some(bx.read()?);
                }

                BoxType::MetaBox => {
                    meta = Some(bx.read()?);
                }

                BoxType::MvexBox => {
                    mvex = Some(bx.read()?);
                }

                BoxType::TrakBox => {
                    traks.push(bx.read()?);
                }

                BoxType::UdtaBox => {
                    udta = Some(bx.read()?);
                }

                _ => continue,
            }
        }

        if mvhd.is_none() {
            return Err(Error::BoxNotFound(BoxType::MvhdBox));
        }

        println!("end reading moov");
        Ok(MoovBox {
            mvhd: mvhd.unwrap(),
            meta,
            udta,
            mvex,
            traks,
        })
    }

    fn size_hint() -> usize {
        MvhdBox::size_hint()
    }
}

impl<W: Write> WriteBox<&mut W> for MoovBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        self.mvhd.write_box(writer)?;
        for trak in self.traks.iter() {
            trak.write_box(writer)?;
        }
        if let Some(meta) = &self.meta {
            meta.write_box(writer)?;
        }
        if let Some(udta) = &self.udta {
            udta.write_box(writer)?;
        }
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;

    #[test]
    fn test_moov() {
        let src_box = MoovBox {
            mvhd: MvhdBox::default(),
            mvex: None, // XXX mvex is not written currently
            traks: vec![],
            meta: Some(MetaBox::default()),
            udta: Some(UdtaBox::default()),
        };

        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MoovBox);
        assert_eq!(header.size, src_box.box_size());

        let dst_box = MoovBox::read_block(&mut reader).unwrap();
        assert_eq!(dst_box, src_box);
    }

    #[test]
    fn test_moov_empty() {
        let src_box = MoovBox::default();

        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = buf.as_slice();
        let header = BoxHeader::read_sync(&mut reader).unwrap().unwrap();
        assert_eq!(header.kind, BoxType::MoovBox);
        assert_eq!(header.size, src_box.box_size());

        let dst_box = MoovBox::read_block(&mut reader).unwrap();
        assert_eq!(dst_box, src_box);
    }
}

use std::collections::HashMap;

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{BlockReader, BoxHeader, BoxType, EmsgBox, Error, FtypBox, MoofBox, MoovBox, Mp4Track};

#[derive(Debug, Clone)]
pub struct Mp4Header {
    pub ftyp: Option<FtypBox>,
    pub moov: Option<MoovBox>,
    pub moofs: Vec<MoofBox>,
    pub emsgs: Vec<EmsgBox>,
    pub data: Vec<(u64, u64)>,
}

impl Mp4Header {
    pub async fn read_until_mdat<R, C>(reader: &mut R) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut offset = 0;
        let mut ftyp = None;
        let mut moov = None;
        let mut moofs = Vec::new();
        // let mut moof_offsets = Vec::new();
        let mut emsgs = Vec::new();
        let mut buff = Vec::with_capacity(8192);

        while let Some(BoxHeader { kind, size: s }) = BoxHeader::read(reader).await? {
            if buff.len() < s as usize {
                buff.resize(s as usize, 0);
            }

            // Match and parse the atom boxes.
            match kind {
                BoxType::FtypBox => {
                    reader.read_exact(&mut buff[0..s as usize]).await?;
                    ftyp = Some(FtypBox::read_block(&mut &buff[0..s as usize])?);
                }

                BoxType::MoovBox => {
                    reader.read_exact(&mut buff[0..s as usize]).await?;
                    moov = Some(MoovBox::read_block(&mut &buff[0..s as usize])?);
                }

                BoxType::MoofBox => {
                    let moof_offset = reader.stream_position()? - 8;
                    let moof = MoofBox::read_box(reader, s)?;
                    moofs.push(moof);
                    moof_offsets.push(moof_offset);
                }

                BoxType::EmsgBox => {
                    let emsg = EmsgBox::read_box(reader, s)?;
                    emsgs.push(emsg);
                }
                BoxType::MdatBox => {}

                // BoxType::FreeBox => {
                //     reader.read_exact(buf)
                //     skip_box(reader, s)?;
                // }
                bt => {
                    println!("skip {:?}", bt);

                    let mut buff = [0u8; 1024];
                    let mut read = 0;
                    for chunk in (0..s).step_by(1024) {
                        if chunk == 0 {
                            continue;
                        }

                        reader.read_exact(&mut buff).await?;
                        read += buff.len();
                    }

                    if s as usize - read > 0 {
                        reader.read_exact(&mut buff[0..s as usize - read]).await?;
                    }
                }
            }
        }

        if ftyp.is_none() {
            return Err(Error::BoxNotFound(BoxType::FtypBox));
        }

        if moov.is_none() {
            return Err(Error::BoxNotFound(BoxType::MoovBox));
        }

        let mut tracks = if let Some(ref moov) = moov {
            if moov.traks.iter().any(|trak| trak.tkhd.track_id == 0) {
                return Err(Error::InvalidData("illegal track id 0"));
            }
            moov.traks
                .iter()
                .map(|trak| (trak.tkhd.track_id, Mp4Track::from(trak)))
                .collect()
        } else {
            HashMap::new()
        };

        // Update tracks if any fragmented (moof) boxes are found.
        // if !moofs.is_empty() {
        //     let mut default_sample_duration = 0;
        //     if let Some(ref moov) = moov {
        //         if let Some(ref mvex) = &moov.mvex {
        //             default_sample_duration = mvex.trex.default_sample_duration
        //         }
        //     }

        //     for (moof, moof_offset) in moofs.iter().zip(moof_offsets) {
        //         for traf in moof.trafs.iter() {
        //             let track_id = traf.tfhd.track_id;
        //             if let Some(track) = tracks.get_mut(&track_id) {
        //                 track.default_sample_duration = default_sample_duration;
        //                 track.moof_offsets.push(moof_offset);
        //                 track.trafs.push(traf.clone())
        //             } else {
        //                 return Err(Error::TrakNotFound(track_id));
        //             }
        //         }
        //     }
        // }

        Ok(Mp4Header {
            ftyp,
            moov,
            moofs,
            emsgs,
            tracks,
        })
    }

    pub fn can_be_streamed(&self) -> bool {
        self.moov.is_some()
    }
}

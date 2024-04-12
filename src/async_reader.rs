use std::{
    collections::HashMap,
    io::{Read, Seek},
    time::Duration,
};

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    BlockReader, BoxHeader, BoxType, EmsgBox, Error, FourCC, FtypBox, MetaBox, Metadata, MoofBox,
    MoovBox, Mp4Sample, Mp4Track,
};

#[derive(Debug, Clone)]
pub struct Mp4Header {
    pub ftyp: Option<FtypBox>,
    pub moov: Option<MoovBox>,
    pub moofs: Vec<MoofBox>,
    pub emsgs: Vec<EmsgBox>,

    tracks: HashMap<u32, Mp4Track>,
}

// async fn read

impl Mp4Header {
    pub async fn read<R, C>(reader: &mut R, _cache: Option<C>) -> Result<Self, Error>
    where
        R: AsyncRead + Unpin,
        // C: AsyncRead + AsyncWrite + Unpin,
    {
        let mut ftyp = None;
        let mut moov = None;
        let mut moofs = Vec::new();
        // let mut moof_offsets = Vec::new();
        let mut emsgs = Vec::new();
        let mut buff = Vec::with_capacity(1024);

        while let Some(BoxHeader { kind, size: s }) = BoxHeader::read(reader).await? {
            if buff.len() < s as usize {
                buff.resize(s as usize, 0);
            }

            // Match and parse the atom boxes.
            match kind {
                BoxType::FtypBox => {
                    reader.read_exact(&mut buff[0..s as usize]).await?;

                    ftyp = Some(FtypBox::read_block(&mut &buff[0..s as usize])?);
                    println!("{:?}", ftyp);
                }

                BoxType::MoovBox => {
                    reader.read_exact(&mut buff[0..s as usize]).await?;

                    println!("moov");

                    moov = Some(MoovBox::read_block(&mut &buff[0..s as usize])?);
                }

                // BoxType::MoofBox => {
                //     let moof_offset = reader.stream_position()? - 8;
                //     let moof = MoofBox::read_box(reader, s)?;
                //     moofs.push(moof);
                //     moof_offsets.push(moof_offset);
                // }

                // BoxType::EmsgBox => {
                //     let emsg = EmsgBox::read_box(reader, s)?;
                //     emsgs.push(emsg);
                // }
                // BoxType::MdatBox => {
                // skip_box(reader, s)?;
                // }

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

    #[inline]
    pub fn major_brand(&self) -> Option<&FourCC> {
        Some(&self.ftyp.as_ref()?.major_brand)
    }

    pub fn minor_version(&self) -> Option<u32> {
        Some(self.ftyp.as_ref()?.minor_version)
    }

    pub fn compatible_brands(&self) -> Option<&[FourCC]> {
        Some(&self.ftyp.as_ref()?.compatible_brands)
    }

    pub fn duration(&self) -> Option<Duration> {
        self.moov.as_ref().map(|moov| {
            Duration::from_millis(moov.mvhd.duration * 1000 / moov.mvhd.timescale as u64)
        })
    }

    pub fn timescale(&self) -> Option<u32> {
        Some(self.moov.as_ref()?.mvhd.timescale)
    }

    pub fn is_fragmented(&self) -> bool {
        !self.moofs.is_empty()
    }

    pub fn tracks(&self) -> &HashMap<u32, Mp4Track> {
        &self.tracks
    }

    pub fn sample_count(&self, track_id: u32) -> Result<u32, Error> {
        if let Some(track) = self.tracks.get(&track_id) {
            Ok(track.sample_count())
        } else {
            Err(Error::TrakNotFound(track_id))
        }
    }

    pub fn read_sample<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        track_id: u32,
        sample_id: u32,
    ) -> Result<Option<Mp4Sample>, Error> {
        if let Some(track) = self.tracks.get(&track_id) {
            track.read_sample(reader, sample_id)
        } else {
            Err(Error::TrakNotFound(track_id))
        }
    }

    pub fn sample_offset(&mut self, track_id: u32, sample_id: u32) -> Result<u64, Error> {
        if let Some(track) = self.tracks.get(&track_id) {
            track.sample_offset(sample_id)
        } else {
            Err(Error::TrakNotFound(track_id))
        }
    }

    pub fn metadata(&self) -> Option<impl Metadata<'_>> {
        self.moov.as_ref()?.udta.as_ref().and_then(|udta| {
            udta.meta.as_ref().and_then(|meta| match meta {
                MetaBox::Mdir { ilst } => ilst.as_ref(),
                _ => None,
            })
        })
    }
}

#[derive(Debug)]
pub struct AsyncMp4Reader<R> {
    pub header: Mp4Header,
    reader: R,
}

impl<R: AsyncRead + Unpin> AsyncMp4Reader<R> {
    pub fn from_reader(reader: R, header: Mp4Header) -> Self {
        Self { reader, header }
    }

    pub async fn read_header(mut reader: R) -> Result<Self, Error> {
        Ok(AsyncMp4Reader {
            header: Mp4Header::read(&mut reader, Some(())).await?,
            reader,
        })
    }

    // pub fn read_fragment_header<FR: Read + Seek>(
    //     &self,
    //     mut reader: FR,
    //     size: u64,
    // ) -> Result<Mp4Reader<FR>> {
    //     Ok(Mp4Reader {
    //         header: self.header.read_fragment(&mut reader, size)?,
    //         reader,
    //     })
    // }

    // pub fn size(&self) -> u64 {
    //     self.header.size()
    // }

    pub fn major_brand(&self) -> Option<&FourCC> {
        self.header.major_brand()
    }

    pub fn minor_version(&self) -> Option<u32> {
        self.header.minor_version()
    }

    pub fn compatible_brands(&self) -> Option<&[FourCC]> {
        self.header.compatible_brands()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.header.duration()
    }

    pub fn timescale(&self) -> Option<u32> {
        self.header.timescale()
    }

    pub fn is_fragmented(&self) -> bool {
        self.header.is_fragmented()
    }

    pub fn tracks(&self) -> &HashMap<u32, Mp4Track> {
        self.header.tracks()
    }

    pub fn sample_count(&self, track_id: u32) -> Result<u32, Error> {
        self.header.sample_count(track_id)
    }

    pub fn read_sample(
        &mut self,
        track_id: u32,
        sample_id: u32,
    ) -> Result<Option<Mp4Sample>, Error> {
        self.header
            .read_sample(&mut self.reader, track_id, sample_id)
    }

    pub fn sample_offset(&mut self, track_id: u32, sample_id: u32) -> Result<u64, Error> {
        self.header.sample_offset(track_id, sample_id)
    }
}

pub struct Mp4Track {}

impl<R> AsyncMp4Reader<R> {
    pub fn metadata(&self) -> impl Metadata<'_> {
        self.header.metadata()
    }
}

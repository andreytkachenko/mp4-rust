use futures::{Future, Stream};
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

use crate::{
    BlockReader, BoxHeader, BoxType, EmsgBox, Error, FtypBox, MoofBox, MoovBox, Mp4Sample,
};

pub struct MemoryBufferProvider {
    buffers: Vec<Vec<u8>>,
}

impl BufferProvider for MemoryBufferProvider {
    type Error = ();
    type Buffer = MemoryBuffer;

    fn create_buffer(&mut self, size: usize) -> Result<Self::Buffer, Self::Error> {
        todo!()
    }
}

pub struct MemoryBuffer {
    inner: Vec<u8>,
}

impl Buffer for MemoryBuffer {
    type Error = std::io::Error;

    async fn write_bytes(
        &mut self,
        reader: &mut (impl AsyncRead + Unpin),
        bytes: usize,
    ) -> Result<(), Self::Error> {
        self.inner.resize(bytes, 0);
        reader.read_exact(&mut self.inner).await?;
        Ok(())
    }

    async fn read_bytes(&self, offset: usize, len: usize) -> Result<&[u8], Self::Error> {
        Ok(self.inner.get(offset..offset + len).unwrap())
    }
}

pub trait Buffer {
    type Error;

    fn write_bytes(
        &mut self,
        reader: &mut (impl AsyncRead + Unpin),
        bytes: usize,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    fn read_bytes(
        &self,
        offset: usize,
        len: usize,
    ) -> impl Future<Output = Result<&[u8], Self::Error>>;
}

pub trait BufferProvider {
    type Error;
    type Buffer: Buffer;

    fn create_buffer(&mut self, size: usize) -> Result<Self::Buffer, Self::Error>;
}

struct DataBlock<B: AsRef<[u8]>> {
    index: usize,
    offset: u64,
    size: u64,
    buffer: B,
}

pub struct Mp4File<'a, R: AsyncRead + Unpin> {
    // provider: P,
    // data: Vec<DataBlock<P::Buffer>>,
    ftyp: Option<FtypBox>,
    emsgs: Vec<EmsgBox>,
    mdat_offset: u64,
    mdat_size: u64,
    tracks: HashMap<u32, Mp4Track>,
    reader: OffsetWrapper<&'a mut R>,
}

pin_project_lite::pin_project! {
pub struct OffsetWrapper<R: AsyncRead> {
    #[pin]
    inner: R,
    pub offset: u64,
}}

impl<R: AsyncRead + Unpin> AsyncRead for OffsetWrapper<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut this = self.project();
        match this.inner.poll_read(cx, buf) {
            Poll::Ready(re) => {
                *this.offset += buf.filled().len() as u64;

                Poll::Ready(re)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<'a, R: AsyncRead + Unpin + 'a> Mp4File<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Self {
            ftyp: None,
            emsgs: Vec::new(),
            mdat_offset: 0,
            mdat_size: 0,
            tracks: HashMap::new(),
            reader: OffsetWrapper {
                inner: reader,
                offset: 0,
            },
        }
    }

    pub async fn read_header(&mut self) -> Result<bool, Error> {
        let mut buff = Vec::with_capacity(8192);
        let mut got_moov = false;

        while let Some(BoxHeader { kind, size: s }) = BoxHeader::read(&mut self.reader).await? {
            match kind {
                BoxType::FtypBox => {
                    if buff.len() < s as usize {
                        buff.resize(s as usize, 0);
                    }

                    self.reader.read_exact(&mut buff[0..s as usize]).await?;
                    self.ftyp = Some(FtypBox::read_block(&mut &buff[0..s as usize])?);
                }

                BoxType::MoovBox => {
                    if buff.len() < s as usize {
                        buff.resize(s as usize, 0);
                    }

                    self.reader.read_exact(&mut buff[0..s as usize]).await?;
                    got_moov = true;
                    self.set_moov(MoovBox::read_block(&mut &buff[0..s as usize])?)?;
                }

                BoxType::MoofBox => {
                    if buff.len() < s as usize {
                        buff.resize(s as usize, 0);
                    }

                    let offset = self.reader.offset;
                    self.reader.read_exact(&mut buff[0..s as usize]).await?;
                    self.add_moof(offset, MoofBox::read_block(&mut &buff[0..s as usize])?)?;
                }

                BoxType::EmsgBox => {
                    if buff.len() < s as usize {
                        buff.resize(s as usize, 0);
                    }

                    self.reader.read_exact(&mut buff[0..s as usize]).await?;
                    self.emsgs
                        .push(EmsgBox::read_block(&mut &buff[0..s as usize])?);
                }

                BoxType::MdatBox => {
                    self.mdat_offset = self.reader.offset;
                    self.mdat_size = s;

                    break;
                }

                bt => {
                    self.skip_box(bt, s).await?;
                }
            }
        }

        Ok(got_moov)
    }

    async fn skip_box(&mut self, bt: BoxType, s: u64) -> Result<(), Error> {
        println!("skip {:?}", bt);

        let mut buff = [0u8; 1024];
        let mut read = 0;
        for chunk in (0..s).step_by(1024) {
            if chunk == 0 {
                continue;
            }

            self.reader.read_exact(&mut buff).await?;
            read += buff.len();
        }

        if s as usize - read > 0 {
            self.reader
                .read_exact(&mut buff[0..s as usize - read])
                .await?;
        }

        Ok(())
    }

    fn set_moov(&mut self, moov: MoovBox) -> Result<(), Error> {
        for trak in moov.traks {
            self.tracks.insert(trak.tkhd.track_id, Mp4Track::new(trak)?);
        }

        Ok(())
    }

    fn add_moof(&mut self, offset: u64, moof: MoofBox) -> Result<(), Error> {
        for traf in moof.trafs {
            let track_id = traf.tfhd.track_id;

            if let Some(track) = self.tracks.get_mut(&track_id) {
                track.add_traf(traf)
            } else {
                return Err(Error::TrakNotFound(track_id));
            }
        }

        Ok(())
    }

    // pub fn into_stream(mut self) -> impl Stream<Item = Result<Mp4Sample, Error>> + 'a {
    //     async_stream::try_stream! {

    //         let mut buff = Vec::with_capacity(8192);
    //         while let Some(BoxHeader { kind, size: s }) = BoxHeader::read(&mut self.reader).await? {
    //             match kind {
    //                 BoxType::FtypBox => {
    //                     if buff.len() < s as usize {
    //                         buff.resize(s as usize, 0);
    //                     }

    //                     self.reader.read_exact(&mut buff[0..s as usize]).await?;
    //                     self.ftyp = Some(FtypBox::read_block(&mut &buff[0..s as usize])?);
    //                 }

    //                 BoxType::MoovBox => {
    //                     if buff.len() < s as usize {
    //                         buff.resize(s as usize, 0);
    //                     }

    //                     self.reader.read_exact(&mut buff[0..s as usize]).await?;
    //                     self.set_moov(MoovBox::read_block(&mut &buff[0..s as usize])?)?;
    //                 }

    //                 BoxType::MoofBox => {
    //                     if buff.len() < s as usize {
    //                         buff.resize(s as usize, 0);
    //                     }

    //                     let offset = self.reader.offset;
    //                     self.reader.read_exact(&mut buff[0..s as usize]).await?;
    //                     self.add_moof(offset, MoofBox::read_block(&mut &buff[0..s as usize])?)?;
    //                 }

    //                 BoxType::EmsgBox => {
    //                     if buff.len() < s as usize {
    //                         buff.resize(s as usize, 0);
    //                     }

    //                     self.reader.read_exact(&mut buff[0..s as usize]).await?;
    //                     self.emsgs.push(EmsgBox::read_block(&mut &buff[0..s as usize])?);
    //                 }

    //                 BoxType::MdatBox => {

    //                     break;
    //                 }

    //                 bt => {
    //                     self.skip_box(bt, s).await?;
    //                 }
    //             }
    //         }
    //     }
    // }
}

struct Mp4SampleOffset {
    offset: u64,
    size: u32,
    duration: u32,
    start_time: u64,
    rendering_offset: i32,
    is_sync: bool,
}

pub struct Mp4Track {
    track_id: u32,
    duration: u64,
    default_sample_duration: u32,
    samples: Vec<Mp4SampleOffset>,
    tkhd: crate::TkhdBox,
}

impl Mp4Track {
    fn new(trak: crate::TrakBox) -> Result<Mp4Track, Error> {
        let mut samples = Vec::with_capacity(trak.mdia.minf.stbl.stsz.sample_count as _);

        for sample_id in 0..trak.mdia.minf.stbl.stsz.sample_count {
            let offset = match trak.sample_offset(sample_id) {
                Ok(offset) => offset,
                Err(Error::EntryInStblNotFound(_, _, _)) => continue,
                Err(err) => return Err(err),
            };

            let size = match trak.sample_size(sample_id) {
                Ok(size) => size,
                Err(Error::EntryInStblNotFound(_, _, _)) => continue,
                Err(err) => return Err(err),
            };

            let (start_time, duration) = trak.sample_time(sample_id)?;

            samples.push(Mp4SampleOffset {
                offset,
                duration,
                size,
                start_time,
                rendering_offset: trak.sample_rendering_offset(sample_id),
                is_sync: trak.sample_is_sync(sample_id),
            });
        }

        Ok(Self {
            tkhd: trak.tkhd,
            track_id: trak.tkhd.track_id,
            default_sample_duration: 0,
            samples,
            duration: 0,
        })
    }

    pub(crate) fn add_traf(&self, traf: crate::TrafBox) {
        todo!()
    }
}

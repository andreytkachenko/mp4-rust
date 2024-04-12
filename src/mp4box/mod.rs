//! All ISO-MP4 boxes (atoms) and operations.
//!
//! * [ISO/IEC 14496-12](https://en.wikipedia.org/wiki/MPEG-4_Part_14) - ISO Base Media File Format (QuickTime, MPEG-4, etc)
//! * [ISO/IEC 14496-14](https://en.wikipedia.org/wiki/MPEG-4_Part_14) - MP4 file format
//! * ISO/IEC 14496-17 - Streaming text format
//! * [ISO 23009-1](https://www.iso.org/standard/79329.html) -Dynamic adaptive streaming over HTTP (DASH)
//!
//! http://developer.apple.com/documentation/QuickTime/QTFF/index.html
//! http://www.adobe.com/devnet/video/articles/mp4_movie_atom.html
//! http://mp4ra.org/#/atoms
//!
//! Supported Atoms:
//! ftyp
//! moov
//!     mvhd
//!     udta
//!         meta
//!             ilst
//!                 data
//!     trak
//!         tkhd
//!         mdia
//!             mdhd
//!             hdlr
//!             minf
//!                 stbl
//!                     stsd
//!                         avc1
//!                         hev1
//!                         mp4a
//!                         tx3g
//!                     stts
//!                     stsc
//!                     stsz
//!                     stss
//!                     stco
//!                     co64
//!                     ctts
//!                 dinf
//!                     dref
//!                 smhd
//!                 vmhd
//!         edts
//!             elst
//!     mvex
//!         mehd
//!         trex
//! emsg
//! moof
//!     mfhd
//!     traf
//!         tfhd
//!         tfdt
//!         trun
//! mdat
//! free
//!

use byteorder::{BigEndian, ByteOrder, WriteBytesExt};
use bytes::Buf;
use std::io::Write;
use std::{convert::TryInto, marker::PhantomData};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::*;

pub(crate) mod avc1;
pub(crate) mod co64;
pub(crate) mod ctts;
pub(crate) mod data;
pub(crate) mod dinf;
pub(crate) mod edts;
pub(crate) mod elst;
pub(crate) mod emsg;
pub(crate) mod ftyp;
pub(crate) mod hdlr;
pub(crate) mod hev1;
pub(crate) mod ilst;
pub(crate) mod mdhd;
pub(crate) mod mdia;
pub(crate) mod mehd;
pub(crate) mod meta;
pub(crate) mod mfhd;
pub(crate) mod minf;
pub(crate) mod moof;
pub(crate) mod moov;
pub(crate) mod mp4a;
pub(crate) mod mvex;
pub(crate) mod mvhd;
pub(crate) mod smhd;
pub(crate) mod stbl;
pub(crate) mod stco;
pub(crate) mod stsc;
pub(crate) mod stsd;
pub(crate) mod stss;
pub(crate) mod stsz;
pub(crate) mod stts;
pub(crate) mod tfdt;
pub(crate) mod tfhd;
pub(crate) mod tkhd;
pub(crate) mod traf;
pub(crate) mod trak;
pub(crate) mod trex;
pub(crate) mod trun;
pub(crate) mod tx3g;
pub(crate) mod udta;
pub(crate) mod vmhd;
pub(crate) mod vp09;
pub(crate) mod vpcc;

pub use avc1::Avc1Box;
pub use co64::Co64Box;
pub use ctts::CttsBox;
pub use data::DataBox;
pub use dinf::DinfBox;
pub use edts::EdtsBox;
pub use elst::ElstBox;
pub use emsg::EmsgBox;
pub use ftyp::FtypBox;
pub use hdlr::HdlrBox;
pub use hev1::Hev1Box;
pub use ilst::IlstBox;
pub use mdhd::MdhdBox;
pub use mdia::MdiaBox;
pub use mehd::MehdBox;
pub use meta::MetaBox;
pub use mfhd::MfhdBox;
pub use minf::MinfBox;
pub use moof::MoofBox;
pub use moov::MoovBox;
pub use mp4a::Mp4aBox;
pub use mvex::MvexBox;
pub use mvhd::MvhdBox;
pub use smhd::SmhdBox;
pub use stbl::StblBox;
pub use stco::StcoBox;
pub use stsc::StscBox;
pub use stsd::StsdBox;
pub use stss::StssBox;
pub use stsz::StszBox;
pub use stts::SttsBox;
pub use tfdt::TfdtBox;
pub use tfhd::TfhdBox;
pub use tkhd::TkhdBox;
pub use traf::TrafBox;
pub use trak::TrakBox;
pub use trex::TrexBox;
pub use trun::TrunBox;
pub use tx3g::Tx3gBox;
pub use udta::UdtaBox;
pub use vmhd::VmhdBox;
pub use vp09::Vp09Box;
pub use vpcc::VpccBox;

pub const HEADER_SIZE: u64 = 8;
// const HEADER_LARGE_SIZE: u64 = 16;
pub const HEADER_EXT_SIZE: u64 = 4;

macro_rules! boxtype {
    ($( $name:ident => $value:expr ),*) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub enum BoxType {
            $( $name, )*
            UnknownBox(u32),
        }

        impl BoxType {
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( BoxType::$name => stringify!($name), )*
                    BoxType::UnknownBox(_) => "unknown",
                }
            }
        }

        impl From<u32> for BoxType {
            fn from(t: u32) -> BoxType {
                match t {
                    $( $value => BoxType::$name, )*
                    _ => BoxType::UnknownBox(t),
                }
            }
        }

        impl From<BoxType> for u32 {
            fn from(b: BoxType) -> u32 {
                match b {
                    $( BoxType::$name => $value, )*
                    BoxType::UnknownBox(t) => t,
                }
            }
        }
    }
}

boxtype! {
    FtypBox => 0x66747970,
    MvhdBox => 0x6d766864,
    MfhdBox => 0x6d666864,
    FreeBox => 0x66726565,
    MdatBox => 0x6d646174,
    MoovBox => 0x6d6f6f76,
    MvexBox => 0x6d766578,
    MehdBox => 0x6d656864,
    TrexBox => 0x74726578,
    EmsgBox => 0x656d7367,
    MoofBox => 0x6d6f6f66,
    TkhdBox => 0x746b6864,
    TfhdBox => 0x74666864,
    TfdtBox => 0x74666474,
    EdtsBox => 0x65647473,
    MdiaBox => 0x6d646961,
    ElstBox => 0x656c7374,
    MdhdBox => 0x6d646864,
    HdlrBox => 0x68646c72,
    MinfBox => 0x6d696e66,
    VmhdBox => 0x766d6864,
    StblBox => 0x7374626c,
    StsdBox => 0x73747364,
    SttsBox => 0x73747473,
    CttsBox => 0x63747473,
    StssBox => 0x73747373,
    StscBox => 0x73747363,
    StszBox => 0x7374737A,
    StcoBox => 0x7374636F,
    Co64Box => 0x636F3634,
    TrakBox => 0x7472616b,
    TrafBox => 0x74726166,
    TrunBox => 0x7472756E,
    UdtaBox => 0x75647461,
    MetaBox => 0x6d657461,
    DinfBox => 0x64696e66,
    DrefBox => 0x64726566,
    UrlBox  => 0x75726C20,
    SmhdBox => 0x736d6864,
    Avc1Box => 0x61766331,
    AvcCBox => 0x61766343,
    Hev1Box => 0x68657631,
    HvcCBox => 0x68766343,
    Mp4aBox => 0x6d703461,
    EsdsBox => 0x65736473,
    Tx3gBox => 0x74783367,
    VpccBox => 0x76706343,
    Vp09Box => 0x76703039,
    DataBox => 0x64617461,
    IlstBox => 0x696c7374,
    NameBox => 0xa96e616d,
    DayBox => 0xa9646179,
    CovrBox => 0x636f7672,
    DescBox => 0x64657363,
    WideBox => 0x77696465,
    WaveBox => 0x77617665
}

pub trait Mp4Box: Sized {
    const TYPE: BoxType;

    fn box_size(&self) -> u64;
    fn to_json(&self) -> Result<String>;
    fn summary(&self) -> Result<String>;
}

pub struct BoxReader<'a, R: Reader<'a>> {
    kind: BoxType,
    inner: R,
    m: PhantomData<&'a ()>,
}

impl<'a, R: Reader<'a>> BoxReader<'a, R> {
    #[inline]
    pub fn try_read<T: Mp4Box + BlockReader>(&mut self) -> Result<Option<T>> {
        if T::TYPE == self.kind {
            Ok(Some(T::read_block(&mut self.inner)?))
        } else {
            Ok(None)
        }
    }

    #[inline]
    pub fn read<T: Mp4Box + BlockReader>(&mut self) -> Result<T> {
        if T::TYPE == self.kind {
            T::read_block(&mut self.inner)
        } else {
            Err(Error::BoxNotFound(T::TYPE))
        }
    }
}

pub trait Reader<'a> {
    fn take(&mut self, size: usize) -> Result<impl Reader<'a> + '_>;
    fn remaining(&self) -> usize;
    fn skip(&mut self, size: usize);

    fn get_u8(&mut self) -> u8;
    fn get_u16(&mut self) -> u16;
    fn get_u24(&mut self) -> u32;
    fn get_u32(&mut self) -> u32;
    fn get_u48(&mut self) -> u64;
    fn get_u64(&mut self) -> u64;

    fn get_i8(&mut self) -> i8;
    fn get_i16(&mut self) -> i16;
    fn get_i24(&mut self) -> i32;
    fn get_i32(&mut self) -> i32;
    fn get_i48(&mut self) -> i64;
    fn get_i64(&mut self) -> i64;

    #[inline]
    fn try_get_u8(&mut self) -> Result<u8> {
        if self.remaining() < 1 {
            Err(Error::InvalidData("expected at least 1 byte more"))
        } else {
            Ok(self.get_u8())
        }
    }
    #[inline]
    fn try_get_u16(&mut self) -> Result<u16> {
        if self.remaining() < 2 {
            Err(Error::InvalidData("expected at least 2 byte more"))
        } else {
            Ok(self.get_u16())
        }
    }
    #[inline]
    fn try_get_u24(&mut self) -> Result<u32> {
        if self.remaining() < 3 {
            Err(Error::InvalidData("expected at least 3 byte more"))
        } else {
            Ok(self.get_u24())
        }
    }
    #[inline]
    fn try_get_u32(&mut self) -> Result<u32> {
        if self.remaining() < 4 {
            Err(Error::InvalidData("expected at least 4 byte more"))
        } else {
            Ok(self.get_u32())
        }
    }
    #[inline]
    fn try_get_u48(&mut self) -> Result<u64> {
        if self.remaining() < 6 {
            Err(Error::InvalidData("expected at least 6 byte more"))
        } else {
            Ok(self.get_u48())
        }
    }
    #[inline]
    fn try_get_u64(&mut self) -> Result<u64> {
        if self.remaining() < 8 {
            Err(Error::InvalidData("expected at least 8 byte more"))
        } else {
            Ok(self.get_u64())
        }
    }

    #[inline]
    fn try_get_i8(&mut self) -> Result<i8> {
        if self.remaining() < 1 {
            Err(Error::InvalidData("expected at least 1 byte more"))
        } else {
            Ok(self.get_i8())
        }
    }
    #[inline]
    fn try_get_i16(&mut self) -> Result<i16> {
        if self.remaining() < 2 {
            Err(Error::InvalidData("expected at least 2 byte more"))
        } else {
            Ok(self.get_i16())
        }
    }
    #[inline]
    fn try_get_i24(&mut self) -> Result<i32> {
        if self.remaining() < 3 {
            Err(Error::InvalidData("expected at least 3 byte more"))
        } else {
            Ok(self.get_i24())
        }
    }
    #[inline]
    fn try_get_i32(&mut self) -> Result<i32> {
        if self.remaining() < 4 {
            Err(Error::InvalidData("expected at least 4 byte more"))
        } else {
            Ok(self.get_i32())
        }
    }
    #[inline]
    fn try_get_i48(&mut self) -> Result<i64> {
        if self.remaining() < 6 {
            Err(Error::InvalidData("expected at least 6 byte more"))
        } else {
            Ok(self.get_i48())
        }
    }
    #[inline]
    fn try_get_i64(&mut self) -> Result<i64> {
        if self.remaining() < 8 {
            Err(Error::InvalidData("expected at least 8 byte more"))
        } else {
            Ok(self.get_i64())
        }
    }
    fn get_null_terminated_string(&mut self) -> String;
    fn collect(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; size];
        self.copy_to_slice(&mut buf)?;

        Ok(buf)
    }

    #[inline]
    fn collect_remaining(&mut self) -> Vec<u8> {
        self.collect(self.remaining()).unwrap()
    }

    fn copy_to_slice(&mut self, slice: &mut [u8]) -> Result<()>;
    fn get_box(&mut self) -> Result<Option<BoxReader<'a, impl Reader<'a> + '_>>>;

    fn find_box<B: Mp4Box + BlockReader>(&mut self) -> Result<B> {
        self.try_find_box()
            .and_then(|x| x.ok_or_else(|| Error::InvalidData("expected box")))
    }

    fn try_find_box2<A: Mp4Box + BlockReader, B: Mp4Box + BlockReader>(
        &mut self,
    ) -> Result<(Option<A>, Option<B>)> {
        let mut a = None;
        let mut b = None;

        while let Some(mut bx) = self.get_box()? {
            if a.is_none() {
                if let Some(inner) = bx.try_read::<A>()? {
                    a = Some(inner);
                    continue;
                }
            }

            if b.is_none() {
                if let Some(inner) = bx.try_read::<B>()? {
                    b = Some(inner);
                    continue;
                }
            }

            println!(" 1 unknown box {}", bx.kind);
        }

        Ok((a, b))
    }

    fn try_find_box3<A, B, C>(&mut self) -> Result<(Option<A>, Option<B>, Option<C>)>
    where
        A: Mp4Box + BlockReader,
        B: Mp4Box + BlockReader,
        C: Mp4Box + BlockReader,
    {
        let mut a = None;
        let mut b = None;
        let mut c = None;

        while let Some(mut bx) = self.get_box()? {
            if a.is_none() {
                if let Some(inner) = bx.try_read::<A>()? {
                    a = Some(inner);
                    continue;
                }
            }
            if b.is_none() {
                if let Some(inner) = bx.try_read::<B>()? {
                    b = Some(inner);
                    continue;
                }
            }

            if c.is_none() {
                if let Some(inner) = bx.try_read::<C>()? {
                    c = Some(inner);
                    continue;
                }
            }

            println!(" 2 unknown box {}", bx.kind);
        }

        Ok((a, b, c))
    }

    #[inline]
    fn find_box3<A, B, C>(&mut self) -> Result<(A, B, C)>
    where
        A: Mp4Box + BlockReader,
        B: Mp4Box + BlockReader,
        C: Mp4Box + BlockReader,
    {
        let (a, b, c) = self.try_find_box3()?;

        let Some(a) = a else {
            return Err(Error::BoxNotFound(A::TYPE));
        };

        let Some(b) = b else {
            return Err(Error::BoxNotFound(B::TYPE));
        };

        let Some(c) = c else {
            return Err(Error::BoxNotFound(C::TYPE));
        };

        Ok((a, b, c))
    }

    fn try_find_box4<A, B, C, D>(&mut self) -> Result<(Option<A>, Option<B>, Option<C>, Option<D>)>
    where
        A: Mp4Box + BlockReader,
        B: Mp4Box + BlockReader,
        C: Mp4Box + BlockReader,
        D: Mp4Box + BlockReader,
    {
        let mut a = None;
        let mut b = None;
        let mut c = None;
        let mut d = None;

        while let Some(mut bx) = self.get_box()? {
            if a.is_none() {
                if let Some(inner) = bx.try_read::<A>()? {
                    a = Some(inner);
                    continue;
                }
            }

            if b.is_none() {
                if let Some(inner) = bx.try_read::<B>()? {
                    b = Some(inner);
                    continue;
                }
            }

            if c.is_none() {
                if let Some(inner) = bx.try_read::<C>()? {
                    c = Some(inner);
                    continue;
                }
            }

            if d.is_none() {
                if let Some(inner) = bx.try_read::<D>()? {
                    d = Some(inner);
                    continue;
                }
            }

            println!(" 3 unknown box {}", bx.kind);
        }

        Ok((a, b, c, d))
    }

    #[inline]
    fn try_find_box<B: Mp4Box + BlockReader>(&mut self) -> Result<Option<B>> {
        while let Some(mut bx) = self.get_box()? {
            if let Some(inner) = bx.try_read::<B>()? {
                return Ok(Some(inner));
            }

            println!(" 4 unknown box {}", bx.kind);
        }

        Ok(None)
    }
}

impl<'a> Reader<'a> for &'a [u8] {
    #[inline]
    fn take(&mut self, size: usize) -> Result<impl Reader<'a> + '_> {
        if self.len() < size {
            return Err(Error::InvalidData("no bytes left"));
        }

        let buff = &(*self)[0..size];
        self.advance(size);

        Ok(buff)
    }

    #[inline]
    fn skip(&mut self, size: usize) {
        Buf::advance(self, size)
    }

    #[inline]
    fn remaining(&self) -> usize {
        Buf::remaining(self)
    }

    #[inline]
    fn get_u8(&mut self) -> u8 {
        Buf::get_u8(self)
    }

    #[inline]
    fn get_u16(&mut self) -> u16 {
        Buf::get_u16(self)
    }

    #[inline]
    fn get_u24(&mut self) -> u32 {
        let val = BigEndian::read_u24(self.chunk());
        self.skip(3);
        val
    }

    #[inline]
    fn get_u32(&mut self) -> u32 {
        Buf::get_u32(self)
    }

    #[inline]
    fn get_u48(&mut self) -> u64 {
        let val = BigEndian::read_u48(self.chunk());
        self.skip(6);
        val
    }

    #[inline]
    fn get_u64(&mut self) -> u64 {
        Buf::get_u64(self)
    }

    #[inline]
    fn get_i8(&mut self) -> i8 {
        Buf::get_i8(self)
    }

    #[inline]
    fn get_i16(&mut self) -> i16 {
        Buf::get_i16(self)
    }

    #[inline]
    fn get_i24(&mut self) -> i32 {
        todo!()
    }

    #[inline]
    fn get_i32(&mut self) -> i32 {
        Buf::get_i32(self)
    }

    #[inline]
    fn get_i48(&mut self) -> i64 {
        todo!()
    }

    #[inline]
    fn get_i64(&mut self) -> i64 {
        Buf::get_i64(self)
    }

    #[inline]
    fn copy_to_slice(&mut self, slice: &mut [u8]) -> Result<()> {
        if self.len() < slice.len() {
            return Err(Error::InvalidData("expected more bytes"));
        }

        Buf::copy_to_slice(self, slice);

        Ok(())
    }

    #[inline]
    fn get_null_terminated_string(&mut self) -> String {
        let rem = self.len();

        if rem > 0 {
            let size = self.iter().position(|&b| b == b'\0');

            let (size, eat) = if let Some(size) = size {
                (size, size + 1)
            } else {
                (rem, rem)
            };

            let val = String::from_utf8_lossy(&self[0..size]).to_string();
            self.advance(eat);
            val
        } else {
            String::new()
        }
    }

    #[inline]
    fn get_box(&mut self) -> Result<Option<BoxReader<'a, impl Reader<'a> + '_>>> {
        let Some(BoxHeader { kind, size }) = BoxHeader::read_sync(self)? else {
            return Ok(None);
        };

        Ok(Some(BoxReader {
            kind,
            inner: Reader::take(self, size as _)?,
            m: PhantomData,
        }))
    }
}

pub trait BlockReader: Sized {
    fn read_block<'a>(block: &mut impl Reader<'a>) -> Result<Self>;
    fn size_hint() -> usize;
}

pub trait WriteBox<T>: Sized {
    fn write_box(&self, _: T) -> Result<u64>;
}

#[derive(Debug, Clone, Copy)]
pub struct BoxHeader {
    pub kind: BoxType,
    pub size: u64,
}

impl BoxHeader {
    pub fn new(name: BoxType, size: u64) -> Self {
        Self { kind: name, size }
    }

    pub fn read_sync<'a>(reader: &mut impl Reader<'a>) -> Result<Option<Self>> {
        if reader.remaining() < 8 {
            return Ok(None);
        }

        let sz = reader.get_u32();
        let typ = reader.get_u32();

        // Get largesize if size is 1
        let size = if sz == 1 {
            if reader.remaining() < 8 {
                return Err(Error::InvalidData("expected 8 bytes more"));
            }

            let largesize = reader.get_u64();
            // Subtract the length of the serialized largesize, as callers assume `size - HEADER_SIZE` is the length
            // of the box data. Disallow `largesize < 16`, or else a largesize of 8 will result in a BoxHeader::size
            // of 0, incorrectly indicating that the box data extends to the end of the stream.
            match largesize {
                0 => 0,
                1..=15 => return Err(Error::InvalidData("64-bit box size too small")),
                16..=u64::MAX => largesize - 8,
            }
        } else {
            sz as _
        };

        println!(
            "{} box {} {}",
            if sz == 1 { "big" } else { "small" },
            BoxType::from(typ).as_str(),
            size
        );

        Ok(Some(BoxHeader {
            kind: BoxType::from(typ),
            size: size.saturating_sub(HEADER_SIZE),
        }))
    }

    // TODO: if size is 0, then this box is the last one in the file
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Option<Self>> {
        // Create and read to buf.
        let mut buf = [0u8; 8]; // 8 bytes for box header.
        match reader.read_exact(&mut buf).await {
            Ok(_) => (),
            Err(err) => match err.kind() {
                std::io::ErrorKind::UnexpectedEof => return Ok(None),
                _ => return Err(err.into()),
            },
        }

        // Get size.
        let s = buf[0..4].try_into().unwrap();
        let sz = u32::from_be_bytes(s);

        // Get box type string.
        let t = buf[4..8].try_into().unwrap();
        let typ = u32::from_be_bytes(t);

        // Get largesize if size is 1
        let size = if sz == 1 {
            match reader.read_exact(&mut buf).await {
                Ok(_) => (),
                Err(err) => match err.kind() {
                    std::io::ErrorKind::UnexpectedEof => return Ok(None),
                    _ => return Err(err.into()),
                },
            }

            let largesize = u64::from_be_bytes(buf);

            // Subtract the length of the serialized largesize, as callers assume `size - HEADER_SIZE` is the length
            // of the box data. Disallow `largesize < 16`, or else a largesize of 8 will result in a BoxHeader::size
            // of 0, incorrectly indicating that the box data extends to the end of the stream.
            match largesize {
                0 => 0,
                1..=15 => return Err(Error::InvalidData("64-bit box size too small")),
                16..=u64::MAX => largesize - 8,
            }
        } else {
            sz as _
        };

        println!(
            "{} box {} {}",
            if sz == 1 { "big" } else { "small" },
            BoxType::from(typ).as_str(),
            size
        );

        Ok(Some(BoxHeader {
            kind: BoxType::from(typ),
            size: size.saturating_sub(HEADER_SIZE),
        }))
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<u64> {
        if self.size > u32::MAX as u64 {
            writer.write_u32::<BigEndian>(1)?;
            writer.write_u32::<BigEndian>(self.kind.into())?;
            writer.write_u64::<BigEndian>(self.size + HEADER_SIZE)?;
            Ok(16)
        } else {
            writer.write_u32::<BigEndian>((self.size + HEADER_SIZE) as u32)?;
            writer.write_u32::<BigEndian>(self.kind.into())?;
            Ok(8)
        }
    }
}

#[inline]
pub fn read_box_header_ext<'a, R: Reader<'a>>(reader: &mut R) -> (u8, u32) {
    (reader.get_u8(), reader.get_u24())
}

pub fn write_box_header_ext<W: Write>(w: &mut W, v: u8, f: u32) -> Result<u64> {
    w.write_u8(v)?;
    w.write_u24::<BigEndian>(f)?;
    Ok(4)
}

pub fn write_zeros<W: Write>(writer: &mut W, size: u64) -> Result<()> {
    for _ in 0..size {
        writer.write_u8(0)?;
    }
    Ok(())
}

mod value_u32 {
    use crate::types::FixedPointU16;
    use serde::{self, Serializer};

    pub fn serialize<S>(fixed: &FixedPointU16, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(fixed.value())
    }
}

mod value_i16 {
    use crate::types::FixedPointI8;
    use serde::{self, Serializer};

    pub fn serialize<S>(fixed: &FixedPointI8, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i8(fixed.value())
    }
}

mod value_u8 {
    use crate::types::FixedPointU8;
    use serde::{self, Serializer};

    pub fn serialize<S>(fixed: &FixedPointU8, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(fixed.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fourcc() {
        let ftyp_fcc = 0x66747970;
        let ftyp_value = FourCC::from(ftyp_fcc);
        assert_eq!(&ftyp_value.value[..], b"ftyp");
        let ftyp_fcc2: u32 = ftyp_value.into();
        assert_eq!(ftyp_fcc, ftyp_fcc2);
    }

    #[test]
    fn test_largesize_too_small() {
        let error =
            BoxHeader::read_sync(&mut &[0, 0, 0, 1, 1, 2, 3, 4, 0, 0, 0, 0, 0, 0, 0, 7][..]);
        assert!(matches!(error, Err(Error::InvalidData(_))));
    }

    #[test]
    fn test_zero_largesize() {
        let error =
            BoxHeader::read_sync(&mut &[0, 0, 0, 1, 1, 2, 3, 4, 0, 0, 0, 0, 0, 0, 0, 8][..]);
        assert!(matches!(error, Err(Error::InvalidData(_))));
    }

    #[test]
    fn test_nonzero_largesize_too_small() {
        let error =
            BoxHeader::read_sync(&mut &[0, 0, 0, 1, 1, 2, 3, 4, 0, 0, 0, 0, 0, 0, 0, 15][..]);
        assert!(matches!(error, Err(Error::InvalidData(_))));
    }

    #[test]
    fn test_valid_largesize() {
        let header =
            BoxHeader::read_sync(&mut &[0, 0, 0, 1, 1, 2, 3, 4, 0, 0, 0, 0, 0, 0, 0, 16][..]);
        assert!(matches!(header, Ok(Some(BoxHeader { size: 8, .. }))));
    }
}

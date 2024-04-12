use serde::Serialize;
use std::io::Write;

use crate::meta::MetaBox;
use crate::mp4box::*;
use crate::mp4box::{edts::EdtsBox, mdia::MdiaBox, tkhd::TkhdBox};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct TrakBox {
    pub tkhd: TkhdBox,
    pub mdia: MdiaBox,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub edts: Option<EdtsBox>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaBox>,
}

impl TrakBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::TrakBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        size += self.tkhd.box_size();
        if let Some(ref edts) = self.edts {
            size += edts.box_size();
        }
        size += self.mdia.box_size();
        size
    }

    pub(crate) fn stsc_index(&self, sample_id: u32) -> Result<usize> {
        if self.mdia.minf.stbl.stsc.entries.is_empty() {
            return Err(Error::InvalidData("no stsc entries"));
        }

        for (i, entry) in self.mdia.minf.stbl.stsc.entries.iter().enumerate() {
            if sample_id < entry.first_sample {
                return if i == 0 {
                    Err(Error::InvalidData("sample not found"))
                } else {
                    Ok(i - 1)
                };
            }
        }

        Ok(self.mdia.minf.stbl.stsc.entries.len() - 1)
    }

    pub(crate) fn chunk_offset(&self, chunk_id: u32) -> Result<u64> {
        if self.mdia.minf.stbl.stco.is_none() && self.mdia.minf.stbl.co64.is_none() {
            return Err(Error::InvalidData("must have either stco or co64 boxes"));
        }

        if let Some(ref stco) = self.mdia.minf.stbl.stco {
            if let Some(offset) = stco.entries.get(chunk_id as usize - 1) {
                return Ok(*offset as u64);
            } else {
                return Err(Error::EntryInStblNotFound(
                    self.tkhd.track_id,
                    BoxType::StcoBox,
                    chunk_id,
                ));
            }
        } else if let Some(ref co64) = self.mdia.minf.stbl.co64 {
            if let Some(offset) = co64.entries.get(chunk_id as usize - 1) {
                return Ok(*offset);
            } else {
                return Err(Error::EntryInStblNotFound(
                    self.tkhd.track_id,
                    BoxType::Co64Box,
                    chunk_id,
                ));
            }
        }

        Err(Error::Box2NotFound(BoxType::StcoBox, BoxType::Co64Box))
    }

    pub(crate) fn sample_size(&self, sample_id: u32) -> Result<u32> {
        let stsz = &self.mdia.minf.stbl.stsz;

        if stsz.sample_size > 0 {
            return Ok(stsz.sample_size);
        }

        if let Some(size) = stsz.sample_sizes.get(sample_id as usize - 1) {
            Ok(*size)
        } else {
            Err(Error::EntryInStblNotFound(
                self.tkhd.track_id,
                BoxType::StszBox,
                sample_id,
            ))
        }
    }

    pub(crate) fn sample_offset(&self, sample_id: u32) -> Result<u64> {
        let stsc_index = self.stsc_index(sample_id)?;

        let stsc = &self.mdia.minf.stbl.stsc;
        let stsc_entry = stsc.entries.get(stsc_index).unwrap();

        let first_chunk = stsc_entry.first_chunk;
        let first_sample = stsc_entry.first_sample;
        let samples_per_chunk = stsc_entry.samples_per_chunk;

        let chunk_id = sample_id
            .checked_sub(first_sample)
            .map(|n| n / samples_per_chunk)
            .and_then(|n| n.checked_add(first_chunk))
            .ok_or(Error::InvalidData(
                "attempt to calculate stsc chunk_id with overflow",
            ))?;

        let chunk_offset = self.chunk_offset(chunk_id)?;

        let first_sample_in_chunk = sample_id - (sample_id - first_sample) % samples_per_chunk;

        let mut sample_offset = 0;
        for i in first_sample_in_chunk..sample_id {
            sample_offset += self.sample_size(i)?;
        }

        Ok(chunk_offset + sample_offset as u64)
    }

    pub(crate) fn sample_time(&self, sample_id: u32) -> Result<(u64, u32)> {
        let stts = self.mdia.minf.stbl.stts;

        let mut sample_count: u32 = 1;
        let mut elapsed = 0;

        for entry in stts.entries.iter() {
            let new_sample_count =
                sample_count
                    .checked_add(entry.sample_count)
                    .ok_or(Error::InvalidData(
                        "attempt to sum stts entries sample_count with overflow",
                    ))?;

            if sample_id < new_sample_count {
                let start_time =
                    (sample_id - sample_count) as u64 * entry.sample_delta as u64 + elapsed;
                return Ok((start_time, entry.sample_delta));
            }

            sample_count = new_sample_count;
            elapsed += entry.sample_count as u64 * entry.sample_delta as u64;
        }

        Err(Error::EntryInStblNotFound(
            self.tkhd.track_id,
            BoxType::SttsBox,
            sample_id,
        ))
    }

    pub(crate) fn ctts_index(&self, sample_id: u32) -> Result<(usize, u32)> {
        let ctts = self.mdia.minf.stbl.ctts.as_ref().unwrap();
        let mut sample_count: u32 = 1;
        for (i, entry) in ctts.entries.iter().enumerate() {
            let next_sample_count =
                sample_count
                    .checked_add(entry.sample_count)
                    .ok_or(Error::InvalidData(
                        "attempt to sum ctts entries sample_count with overflow",
                    ))?;
            if sample_id < next_sample_count {
                return Ok((i, sample_count));
            }
            sample_count = next_sample_count;
        }

        Err(Error::EntryInStblNotFound(
            self.tkhd.track_id,
            BoxType::CttsBox,
            sample_id,
        ))
    }

    pub fn sample_rendering_offset(self, sample_id: u32) -> i32 {
        if let Some(ref ctts) = self.mdia.minf.stbl.ctts {
            if let Ok((ctts_index, _)) = self.ctts_index(sample_id) {
                let ctts_entry = ctts.entries.get(ctts_index).unwrap();
                return ctts_entry.sample_offset;
            }
        }

        0
    }

    #[inline]
    pub fn sample_is_sync(&self, sample_id: u32) -> bool {
        if let Some(ref stss) = self.mdia.minf.stbl.stss {
            stss.entries.binary_search(&sample_id).is_ok()
        } else {
            true
        }
    }
}

impl Mp4Box for TrakBox {
    const TYPE: BoxType = BoxType::TrakBox;

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

impl BlockReader for TrakBox {
    fn read_block<'a>(reader: &mut impl Reader<'a>) -> Result<Self> {
        let (tkhd, edts, meta, mdia) = reader.try_find_box4()?;

        if tkhd.is_none() {
            return Err(Error::BoxNotFound(BoxType::TkhdBox));
        }

        if mdia.is_none() {
            return Err(Error::BoxNotFound(BoxType::MdiaBox));
        }

        Ok(TrakBox {
            tkhd: tkhd.unwrap(),
            edts,
            meta,
            mdia: mdia.unwrap(),
        })
    }

    fn size_hint() -> usize {
        TkhdBox::size_hint() + MdiaBox::size_hint()
    }
}

impl<W: Write> WriteBox<&mut W> for TrakBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(Self::TYPE, size).write(writer)?;

        self.tkhd.write_box(writer)?;
        if let Some(ref edts) = self.edts {
            edts.write_box(writer)?;
        }
        self.mdia.write_box(writer)?;

        Ok(size)
    }
}

// use std::convert::TryInto;
// use std::env;
// use std::fs::File;
// use std::io::{self, BufReader};
// use std::path::Path;

// use anyhow::bail;
// use bytes::{BufMut, Bytes, BytesMut};
// use futures::SinkExt;
// use mp4::TrackType;
// use std::io::{Cursor, Write};
// use tokio_util::codec::Encoder;

// use bytes::Buf;
// use mpeg2ts::{
//     es::{StreamId, StreamType},
//     pes::PesHeader,
//     time::{ClockReference, Timestamp},
//     ts::{
//         payload::{self, Pat, Pmt},
//         AdaptationField, ContinuityCounter, EsInfo, Pid, ProgramAssociation,
//         TransportScramblingControl, TsHeader, TsPacket, TsPacketWriter, TsPayload, VersionNumber,
//         WriteTsPacket,
//     },
//     Error as TsError,
// };

// const PMT_PID: u16 = 4096;
// const VIDEO_ES_PID: u16 = 256;
// // const AUDIO_ES_PID: u16 = 258;
// const PES_VIDEO_STREAM_ID: u8 = 224;
// // const PES_AUDIO_STREAM_ID: u8 = 192;

// #[derive(Default)]
// pub struct TsEncoder {
//     video_continuity_counter: ContinuityCounter,
//     header_sent: bool,
//     timestamp: i64,
// }

// impl TsEncoder {
//     fn write_packet(
//         &mut self,
//         writer: &mut TsPacketWriter<impl Write>,
//         pts: Timestamp,
//         dts: Timestamp,
//         data: &[u8],
//         is_keyframe: bool,
//     ) -> Result<(), TsError> {
//         let mut header = Self::default_ts_header(VIDEO_ES_PID, self.video_continuity_counter)?;
//         let mut buf = Cursor::new(data);
//         let packet = {
//             let data = payload::Bytes::new(&buf.chunk()[..buf.remaining().min(153)])?;
//             buf.advance(data.len());

//             TsPacket {
//                 header: header.clone(),
//                 adaptation_field: is_keyframe.then(|| AdaptationField {
//                     discontinuity_indicator: false,
//                     random_access_indicator: true,
//                     es_priority_indicator: false,
//                     pcr: Some(ClockReference::from(pts)),
//                     opcr: None,
//                     splice_countdown: None,
//                     transport_private_data: Vec::new(),
//                     extension: None,
//                 }),
//                 payload: Some(TsPayload::Pes(payload::Pes {
//                     header: PesHeader {
//                         stream_id: StreamId::new(PES_VIDEO_STREAM_ID),
//                         priority: false,
//                         data_alignment_indicator: false,
//                         copyright: false,
//                         original_or_copy: false,
//                         pts: Some(pts),
//                         dts: if pts == dts { None } else { Some(dts) },
//                         escr: None,
//                     },
//                     pes_packet_len: 0,
//                     data,
//                 })),
//             }
//         };

//         writer.write_ts_packet(&packet)?;
//         header.continuity_counter.increment();

//         while buf.has_remaining() {
//             let raw_payload =
//                 payload::Bytes::new(&buf.chunk()[..buf.remaining().min(payload::Bytes::MAX_SIZE)])?;

//             buf.advance(raw_payload.len());

//             let packet = TsPacket {
//                 header: header.clone(),
//                 adaptation_field: None,
//                 payload: Some(TsPayload::Raw(raw_payload)),
//             };

//             writer.write_ts_packet(&packet)?;
//             header.continuity_counter.increment();
//         }

//         self.video_continuity_counter = header.continuity_counter;
//         Ok(())
//     }

//     pub fn new(timestamp: i64) -> TsEncoder {
//         Self {
//             video_continuity_counter: Default::default(),
//             header_sent: false,
//             timestamp,
//         }
//     }
// }

// struct Frame {
//     pub pts: i64,
//     pub dts: i64,
//     pub body: Bytes,
//     pub key: bool,
// }

// impl<'a> Encoder<&'a Frame> for TsEncoder {
//     type Error = anyhow::Error;

//     fn encode(&mut self, frame: &'a Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
//         let mut writer = TsPacketWriter::new(dst.writer());

//         if !self.header_sent {
//             self.header_sent = true;
//             self.write_header(&mut writer, StreamType::H264)?;
//         }

//         let pts = frame.pts - self.timestamp;
//         let dts = frame.dts - self.timestamp;
//         let p_ts = Timestamp::new((pts as u64 * 9) / 100 + 1).map_err(TsError::from)?;
//         let d_ts = Timestamp::new((dts as u64 * 9) / 100 + 1).map_err(TsError::from)?;

//         self.write_packet(&mut writer, p_ts, d_ts, &frame.body, frame.key)?;

//         Ok(())
//     }
// }

// impl TsEncoder {
//     #[inline]
//     fn write_header<W: WriteTsPacket>(
//         &mut self,
//         writer: &mut W,
//         stream_type: StreamType,
//     ) -> Result<(), TsError> {
//         self.write_packets(
//             writer,
//             [
//                 &Self::default_pat_packet(),
//                 &Self::default_pmt_packet(stream_type),
//             ],
//         )?;

//         Ok(())
//     }

//     #[inline]
//     fn write_packets<'a, W: WriteTsPacket, P: IntoIterator<Item = &'a TsPacket>>(
//         &mut self,
//         writer: &mut W,
//         packets: P,
//     ) -> Result<(), TsError> {
//         packets
//             .into_iter()
//             .try_for_each(|pak| writer.write_ts_packet(pak))?;

//         Ok(())
//     }

//     fn default_ts_header(
//         pid: u16,
//         continuity_counter: ContinuityCounter,
//     ) -> Result<TsHeader, TsError> {
//         Ok(TsHeader {
//             transport_error_indicator: false,
//             transport_priority: false,
//             pid: Pid::new(pid)?,
//             transport_scrambling_control: TransportScramblingControl::NotScrambled,
//             continuity_counter,
//         })
//     }

//     fn default_pat_packet() -> TsPacket {
//         TsPacket {
//             header: Self::default_ts_header(0, Default::default()).unwrap(),
//             adaptation_field: None,
//             payload: Some(TsPayload::Pat(Pat {
//                 transport_stream_id: 1,
//                 version_number: VersionNumber::default(),
//                 table: vec![ProgramAssociation {
//                     program_num: 1,
//                     program_map_pid: Pid::new(PMT_PID).unwrap(),
//                 }],
//             })),
//         }
//     }

//     fn default_pmt_packet(stream_type: StreamType) -> TsPacket {
//         TsPacket {
//             header: Self::default_ts_header(PMT_PID, Default::default()).unwrap(),
//             adaptation_field: None,
//             payload: Some(TsPayload::Pmt(Pmt {
//                 program_num: 1,
//                 pcr_pid: Some(Pid::new(VIDEO_ES_PID).unwrap()),
//                 version_number: VersionNumber::default(),
//                 program_info: vec![],
//                 es_info: vec![EsInfo {
//                     stream_type,
//                     elementary_pid: Pid::new(VIDEO_ES_PID).unwrap(),
//                     descriptors: vec![],
//                 }],
//             })),
//         }
//     }
// }

// #[tokio::main(flavor = "current_thread")]
// async fn main() {
//     let args: Vec<String> = env::args().collect();

//     if args.len() < 2 {
//         println!("Usage: mp4sample <filename>");
//         std::process::exit(1);
//     }

//     if let Err(err) = samples(&args[1]).await {
//         let _ = writeln!(io::stderr(), "{}", err);
//     }
// }

// async fn samples<P: AsRef<Path>>(filename: &P) -> anyhow::Result<()> {
//     let mut ts_name = filename.as_ref().parent().unwrap().to_path_buf();
//     ts_name.push(format!(
//         "{}.ts",
//         filename.as_ref().file_stem().unwrap().to_str().unwrap()
//     ));

//     let f = File::open(filename)?;
//     let size = f.metadata()?.len();
//     let reader = BufReader::new(f);
//     let ts_file = tokio::fs::File::create(ts_name).await.unwrap();

//     let mut ts = tokio_util::codec::FramedWrite::new(ts_file, TsEncoder::new(-1_400_000));
//     let mut mp4 = mp4::Mp4Reader::read_header(reader, size)?;

//     if let Some(track_id) = mp4.tracks().iter().find_map(|(k, v)| {
//         v.track_type()
//             .ok()
//             .and_then(|x| matches!(x, TrackType::Video).then_some(*k))
//     }) {
//         let sample_count = mp4.sample_count(track_id).unwrap();
//         let mut params = BytesMut::new();
//         let track = mp4.tracks().get(&track_id).unwrap();
//         let timescale = track.timescale();

//         if let Ok(sps) = track.sequence_parameter_set() {
//             params.put_slice(&[0, 0, 0, 1]);
//             params.put_slice(sps);
//         }

//         if let Ok(pps) = track.picture_parameter_set() {
//             params.put_slice(&[0, 0, 0, 1]);
//             params.put_slice(pps);
//         }

//         for sample_idx in 0..sample_count {
//             let sample_id = sample_idx + 1;
//             let sample = mp4.read_sample(track_id, sample_id);

//             if let Some(samp) = sample.unwrap() {
//                 let dts = (samp.start_time as i64 * 1_000_000) / timescale as i64;
//                 let pts = (samp.start_time as i64 + samp.rendering_offset as i64) * 1_000_000
//                     / timescale as i64;

//                 let mut bytes = BytesMut::from(samp.bytes.as_ref());
//                 convert_h264(&mut bytes).unwrap();

//                 let mut body = BytesMut::with_capacity(bytes.len() + 6);

//                 if sample_idx == 0 {
//                     body.put_slice(&params);
//                 }

//                 body.put_slice(&[0, 0, 0, 1, 9, 240]);
//                 body.put_slice(&bytes);

//                 ts.send(&Frame {
//                     pts,
//                     dts,
//                     body: body.freeze(),
//                     key: samp.is_sync,
//                 })
//                 .await?;
//             }
//         }
//     }
//     Ok(())
// }

// fn convert_h264(data: &mut [u8]) -> anyhow::Result<()> {
//     // TODO:
//     // * For each IDR frame, copy the SPS and PPS from the stream's
//     //   parameters, rather than depend on it being present in the frame
//     //   already. In-band parameters aren't guaranteed. This is awkward
//     //   with h264_reader v0.5's h264_reader::avcc::AvcDecoderRecord because it
//     //   strips off the NAL header byte from each parameter. The next major
//     //   version shouldn't do this.
//     // * Copy only the slice data. In particular, don't copy SEI, which confuses
//     //   Safari: <https://github.com/scottlamb/retina/issues/60#issuecomment-1178369955>

//     let mut i = 0;
//     while i < data.len() - 3 {
//         // Replace each NAL's length with the Annex B start code b"\x00\x00\x00\x01".
//         let bytes = &mut data[i..i + 4];
//         let nalu_length = u32::from_be_bytes(bytes.try_into().unwrap()) as usize;
//         bytes.copy_from_slice(&[0, 0, 0, 1]);

//         i += 4 + nalu_length;

//         if i > data.len() {
//             bail!("partial nal body");
//         }
//     }

//     if i < data.len() {
//         bail!("partial nal body");
//     }

//     Ok(())
// }
fn main() {}

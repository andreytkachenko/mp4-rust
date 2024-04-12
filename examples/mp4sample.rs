use std::fmt::Pointer;
use std::io::prelude::*;
use std::path::Path;
use std::{env, io};

use mp4::Result;
use tokio::fs::File;
use tokio::io::BufReader;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: mp4sample <filename>");
        std::process::exit(1);
    }

    if let Err(err) = samples(&args[1]).await {
        let _ = writeln!(io::stderr(), "{}", err);
    }
}

async fn samples<P: AsRef<Path>>(filename: &P) -> Result<()> {
    let f = File::open(filename).await?;
    let reader = BufReader::new(f);
    let mp4 = mp4::AsyncMp4Reader::read_header(reader).await?;

    for track_id in mp4.tracks().keys().copied().collect::<Vec<u32>>() {
        let track = mp4.tracks().get(&track_id).unwrap();

        println!("{:#?}", track.trak.mdia.minf.stbl.stco);

        // let sample_count = mp4.sample_count(track_id).unwrap();

        // println!("sample count {}", sample_count);

        // for sample_idx in 0..sample_count {
        // let sample_id = sample_idx + 1;
        // let sample = mp4.read_sample(track_id, sample_id);

        // if let Some(ref samp) = sample.unwrap() {
        //     println!(
        //         "[{}] start_time={} duration={} rendering_offset={} size={} is_sync={}",
        //         sample_id,
        //         samp.start_time,
        //         samp.duration,
        //         samp.rendering_offset,
        //         samp.bytes.len(),
        //         samp.is_sync,
        //     );
        // }
        // }
    }
    Ok(())
}

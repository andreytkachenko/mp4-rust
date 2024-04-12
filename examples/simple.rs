use mp4::Mp4Header;
use std::env;
use tokio::fs::File;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: simple <filename>");
        std::process::exit(1);
    }

    let filename = &args[1];
    let mut f = File::open(filename).await.unwrap();

    let mp4 = Mp4Header::read(&mut f, Some(())).await.unwrap();

    println!("Major Brand: {:?}", mp4.major_brand());

    for track in mp4.tracks().values() {
        println!(
            "Track: #{}({}) {} {}",
            track.track_id(),
            track.language(),
            track.track_type().unwrap(),
            track.box_type().unwrap(),
        );
    }
}

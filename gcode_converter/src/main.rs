use anyhow::Result;
use clap::Parser;
use inkdrop::gcode::{Calibration, Channel, Channels};
use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(untagged)]
enum ChannelOrChannels {
    Channel(Channel),
    Channels(Channels),
}

#[derive(Parser)]
pub struct Options {
    #[arg(long, short)]
    input: PathBuf,

    #[arg(long, short)]
    output: PathBuf,

    #[arg(long, short)]
    calibration: PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();

    let opt = Options::parse();

    let fh_in = std::fs::File::open(opt.input)?;
    let fh_calib = std::fs::File::open(opt.calibration)?;

    let calib: Calibration = serde_json::from_reader(fh_calib)?;
    let channels: ChannelOrChannels = serde_json::from_reader(fh_in)?;

    let channels = match channels {
        ChannelOrChannels::Channel(c) => vec![c],
        ChannelOrChannels::Channels(c) => c,
    };

    let translated = calib.translate_origin(&channels);
    let transformed = calib.transform_coordinates(&translated);

    std::fs::create_dir_all(&opt.output)?;

    for (index, channel) in transformed.iter().enumerate() {
        let filename = opt.output.join(format!("channel_{index:03}.gcode"));
        let mut fh = std::fs::File::create(&filename)?;
        let gcode = calib.gcode(channel);
        fh.write_all(gcode.as_bytes())?;
    }

    Ok(())
}

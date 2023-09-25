use anyhow::Result;
use inkdrop::point::Point;
use log::info;
use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

use inkdrop::gcode::Calibration;
use inkdrop::gcode::{Channel, Channels};

#[derive(Deserialize)]
#[serde(untagged)]
enum ChannelOrChannels {
    Channel(Channel),
    Channels(Channels),
}

#[derive(StructOpt)]
pub struct Options {
    #[structopt(long, short, parse(from_os_str))]
    input: PathBuf,

    #[structopt(long, short, parse(from_os_str))]
    output: PathBuf,

    #[structopt(long, short, parse(from_os_str))]
    calibration: PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();

    let opt = Options::from_args();

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

    //dbg!(&channels, &translated, &transformed);

    let out_dir = opt.output.as_path().clone();

    std::fs::create_dir_all(out_dir)?;

    for (index, channel) in transformed.iter().enumerate() {
        let file_name: PathBuf = {
            let mut _v = opt.output.clone();
            _v.push(format!("channel_{index:03}.gcode"));
            _v
        };

        let mut fh = std::fs::File::create(&file_name)?;
        let gcode = calib.gcode(channel);
        fh.write_all(gcode.as_bytes())?;
    }

    Ok(())
}

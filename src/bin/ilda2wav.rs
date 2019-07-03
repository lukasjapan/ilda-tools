mod common;

use byteorder::{LittleEndian, WriteBytesExt};
use clap::{App, Arg};
use common::full_buf_writer::FullBufWriter;
use common::memory_cycle::MemoryCycleIterator;
use common::memory_cycle::MemoryCycleIteratorExt;
use hound::{Error as HoundError, WavSpec, WavWriter};
use ilda::animation::{Animation, AnimationFrameIterator, Frame};
use ilda::SimplePoint;
use std::fs::File;
use std::io::{self, BufWriter, Cursor, Error as IoError, Read, Seek, Stdin, Stdout, Write};

trait SampleWrite {
    fn write(&mut self, samples: &Vec<f64>) -> Result<(), IoError>;
    fn finish(self: Box<Self>) -> Result<(), IoError>;
}

enum BytesPerSample {
    OneByte,
    TwoBytes,
    FourBytes,
}

struct PcmWriter<T: Write> {
    writer: T,
    bps: BytesPerSample,
}

// consts for mapping -1.0 ~ 1.0 to min/max values of i8,i16,i32: y = ax + b
const I8A: f64 = (i8::max_value() as f64 - i8::min_value() as f64) / 2.0;
const I8B: f64 = (i8::max_value() as f64 + i8::min_value() as f64) / 2.0;
const I16A: f64 = (i16::max_value() as f64 - i16::min_value() as f64) / 2.0;
const I16B: f64 = (i16::max_value() as f64 + i16::min_value() as f64) / 2.0;
const I32A: f64 = (i32::max_value() as f64 - i32::min_value() as f64) / 2.0;
const I32B: f64 = (i32::max_value() as f64 + i32::min_value() as f64) / 2.0;

impl<T: Write> SampleWrite for PcmWriter<T> {
    fn write(&mut self, samples: &Vec<f64>) -> Result<(), IoError> {
        match self.bps {
            BytesPerSample::OneByte => {
                for sample in samples {
                    self.writer.write_i8((I8A * *sample + I8B) as i8)?
                }
            }
            BytesPerSample::TwoBytes => {
                for sample in samples {
                    self.writer
                        .write_i16::<LittleEndian>((I16A * sample + I16B) as i16)?
                }
            }
            BytesPerSample::FourBytes => {
                for sample in samples {
                    self.writer
                        .write_i32::<LittleEndian>((I32A * sample + I32B) as i32)?
                }
            }
        };

        self.writer.flush()
    }

    fn finish(self: Box<Self>) -> Result<(), IoError> {
        Ok(())
    }
}

struct HoundWriter<W: Write + Seek> {
    hound: WavWriter<W>,
    bps: BytesPerSample,
}

fn map_hound_error(e: HoundError) -> IoError {
    match e {
        HoundError::IoError(e) => e,
        _ => panic!("Unexpected hound error."),
    }
}

impl<W: Write + Seek> SampleWrite for HoundWriter<W> {
    fn write(&mut self, samples: &Vec<f64>) -> Result<(), IoError> {
        match self.bps {
            BytesPerSample::OneByte => {
                for sample in samples {
                    self.hound
                        .write_sample((I8A * *sample + I8B) as i8)
                        .map_err(map_hound_error)?
                }
            }
            BytesPerSample::TwoBytes => {
                for sample in samples {
                    self.hound
                        .write_sample((I16A * sample + I16B) as i16)
                        .map_err(map_hound_error)?
                }
            }
            BytesPerSample::FourBytes => {
                for sample in samples {
                    self.hound
                        .write_sample((I32A * sample + I32B) as i32)
                        .map_err(map_hound_error)?
                }
            }
        };

        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), IoError> {
        self.hound.finalize().map_err(map_hound_error)
    }
}

struct Options {
    input: Box<dyn Read>,
    output: Box<dyn SampleWrite>,
    repeat: bool,
    mdpm: u32,
    fps: f64,
    pps: f64,
    sample_rate: u32,
    correctness: f64,
    channels: Vec<MapConfiguration>,
}

type Mapper = fn(&SimplePoint) -> f64;

struct MapConfiguration {
    mapper: Mapper,
    is_axis: bool,
}

enum Step {
    Linear { from: f64, step: f64 },
    Jump(f64),
}

fn get_options<'a>() -> Options {
    let matches = App::new("ilda2gui")
        .version("0.1.0")
        .author("Lukas <lukasjapan@gmail.com>")
        .about("Generates a wav file for an ILDA projector hooked to a sound card.")
        .arg(
            Arg::with_name("PPS")
                .short("p")
                .long("pps")
                .default_value("10000")
                .help("Point per second of the projector. The maximum limit of points that is sent to the projector per second.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("CORRECTNESS")
                .short("c")
                .long("correctness")
                .default_value("1.0")
                .help(r#"Defines how much time should be used as a minimum per point.
0~1: points may be dropped
1: Guarantee at least pps points per second (default)
1~: Use extra time per point

A value above 1 may lower the pps of your device on frames with a lot of points.
For example a value of 2 will allocate the double amount of time per point, effectively cutting pps in half.

If a frame contains too many points for the projector to handle, a value below 1 allows points to be dropped from rendering.
Points that are close to each other are more likely to be dropped.
Any value above zero may slow down the animation."#)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FPS")
                .short("f")
                .long("fps")
                .default_value("20.0")
                .help("Try to draw this number of frames per second.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("SAMPLERATE")
                .short("s")
                .long("sample-rate")
                .default_value("44100")
                .help("Sample rate of the output wav.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("BPS")
                .short("b")
                .long("bps")
                .default_value("16")
                .help("Bits per sample of the output wav.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("REPEAT")
                .short("r")
                .long("repeat")
                .help("Repeats the input animation forever. Can only be used if outputting raw PCM samples to STDOUT."),
        )
        .arg(
            Arg::with_name("RAW")
                .short("a")
                .long("raw")
                .help("Output raw PCM data. (Do not write wav header)"),
        )
        .arg(
            Arg::with_name("CHANNELS")
                .help(r#"A string that defines the output channel configuration. Use one or more of the following characters:
x: X-Axis
X: X-Axis mirrored
y: Y-Axis
Y: Y-Axis mirrored
r: Intensity of Red component
g: Intensity of Green component
b: Intensity of Blue component
l: Blanking signal
1: Always high
0: Always low
_: Silence

Ex:
A stereo file that controls the axis only: xy
A 5.1 channel file that controls the axis with rear channels and includes the blanking signal: __l_xy
"#)
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("FILES")
                .multiple(true)
                .help(r#"Specify 0~2 filenames.
0 filename: Read the input from STDIN and write the output to STDOUT
1 filename with .ild extension: Read the input from the given file and write the output to STDOUT
1 filename with .wav extension: Read the input from STDIN and write the output to the given file
2 filenames: Read the input from the first file and write the output to the second file

Warning: If writing to STDOUT, the output file will be buffered unless raw PCM samples are requested.
                "#)
                .max_values(2)
                .index(2),
        )
        .arg(
            Arg::with_name("MDPS")
                .short("m")
                .long("mdps")
                .default_value("100")
                .help("Meh - Need to think about how to implement this constraint. This should probably be related to the pps setting")
                .takes_value(true),
        )
        .get_matches();

    let channels = {
        let result: Result<Vec<_>, _> = matches
            .value_of("CHANNELS")
            .unwrap()
            .chars()
            .map(|v| match v {
                'x' => Ok(MapConfiguration {
                    mapper: map_x,
                    is_axis: true,
                }),
                'X' => Ok(MapConfiguration {
                    mapper: map_x_inv,
                    is_axis: true,
                }),
                'y' => Ok(MapConfiguration {
                    mapper: map_y,
                    is_axis: true,
                }),
                'Y' => Ok(MapConfiguration {
                    mapper: map_y_inv,
                    is_axis: true,
                }),
                'r' => Ok(MapConfiguration {
                    mapper: map_r,
                    is_axis: false,
                }),
                'g' => Ok(MapConfiguration {
                    mapper: map_g,
                    is_axis: false,
                }),
                'b' => Ok(MapConfiguration {
                    mapper: map_b,
                    is_axis: false,
                }),
                'l' => Ok(MapConfiguration {
                    mapper: map_l,
                    is_axis: false,
                }),
                '1' => Ok(MapConfiguration {
                    mapper: map_on,
                    is_axis: false,
                }),
                '0' => Ok(MapConfiguration {
                    mapper: map_off,
                    is_axis: false,
                }),
                '_' => Ok(MapConfiguration {
                    mapper: map_none,
                    is_axis: false,
                }),
                _ => Err(()),
            })
            .collect();

        result.expect("Invalid Channel.")
    };

    let files: Vec<&str> = match matches.values_of("FILES") {
        Some(files) => files.collect(),
        None => vec![],
    };

    let file_in = match files.len() {
        0 => None,
        1 => {
            if files[0].to_lowercase().ends_with(".ild") {
                Some(files[0])
            } else {
                None
            }
        }
        2 => Some(files[0]),
        _ => panic!("This should never happen."),
    };
    let file_out = match files.len() {
        0 => None,
        1 => {
            if files[0].to_lowercase().ends_with(".wav") {
                Some(files[0])
            } else {
                None
            }
        }
        2 => Some(files[1]),
        _ => panic!("This should never happen."),
    };

    if files.len() == 1 && file_in.is_none() && file_out.is_none() {
        panic!("Failed to determine if given file is meant for input or output.")
    }

    let raw_pcm = matches.is_present("RAW");

    let sample_rate: u32 = matches
        .value_of("SAMPLERATE")
        .unwrap()
        .parse()
        .expect("Invalid number.");

    let bits_per_sample: u32 = matches
        .value_of("BPS")
        .unwrap()
        .parse()
        .expect("Invalid number.");

    let bits_per_sample_enum = match bits_per_sample {
        8 => BytesPerSample::OneByte,
        16 => BytesPerSample::TwoBytes,
        32 => BytesPerSample::FourBytes,
        _ => panic!("Invalid sample rate."),
    };

    let bits_per_sample: u16 = matches
        .value_of("BPS")
        .unwrap()
        .parse()
        .expect("Invalid number.");

    let input: Box<Read> = match file_in {
        Some(file) => Box::new(File::open(file).expect("Failed to open file.")),
        None => Box::new(io::stdin()),
    };

    let repeat = matches.is_present("REPEAT");

    let output: Box<SampleWrite> = if raw_pcm {
        match file_out {
            Some(filename) => {
                let writer = BufWriter::new(File::create(filename).expect("Failed to open file."));
                Box::new(PcmWriter {
                    writer,
                    bps: bits_per_sample_enum,
                })
            }
            None => {
                let writer = io::stdout();
                Box::new(PcmWriter {
                    writer,
                    bps: bits_per_sample_enum,
                })
            }
        }
    } else {
        let spec = WavSpec {
            channels: channels.len() as u16,
            sample_rate,
            bits_per_sample,
            sample_format: hound::SampleFormat::Int,
        };

        match file_out {
            Some(filename) => {
                let hound = WavWriter::create(filename, spec).expect("Failed to init wav.");
                Box::new(HoundWriter {
                    hound,
                    bps: bits_per_sample_enum,
                })
            }
            None => {
                let hound = WavWriter::new(FullBufWriter::new(io::stdout()), spec)
                    .expect("Failed to init wav.");
                Box::new(HoundWriter {
                    hound,
                    bps: bits_per_sample_enum,
                })
            }
        }
    };

    if repeat && !(file_out.is_none() && raw_pcm) {
        panic!("Repeating input is only allowed when outputting raw PCM samples to STDOUT.")
    }

    Options {
        input,
        output,
        repeat,
        mdpm: matches
            .value_of("MDPS")
            .unwrap()
            .parse()
            .expect("Invalid number."),
        fps: matches
            .value_of("FPS")
            .unwrap()
            .parse()
            .expect("Invalid number."),
        pps: matches
            .value_of("PPS")
            .unwrap()
            .parse()
            .expect("Invalid number."),
        correctness: matches
            .value_of("CORRECTNESS")
            .unwrap()
            .parse()
            .expect("Invalid number."),
        sample_rate,
        channels,
    }
}

fn map_x(point: &SimplePoint) -> f64 {
    point.x as f64 / std::i16::MAX as f64
}
fn map_y(point: &SimplePoint) -> f64 {
    point.y as f64 / std::i16::MAX as f64
}
fn map_x_inv(point: &SimplePoint) -> f64 {
    -(point.x as f64 / std::i16::MAX as f64)
}
fn map_y_inv(point: &SimplePoint) -> f64 {
    -(point.y as f64 / std::i16::MAX as f64)
}
fn map_r(point: &SimplePoint) -> f64 {
    point.r as f64 * 2.0 / std::u8::MAX as f64 - 1.0
}
fn map_g(point: &SimplePoint) -> f64 {
    point.g as f64 * 2.0 / std::u8::MAX as f64 - 1.0
}
fn map_b(point: &SimplePoint) -> f64 {
    point.b as f64 * 2.0 / std::u8::MAX as f64 - 1.0
}
fn map_l(point: &SimplePoint) -> f64 {
    if point.is_blank {
        -1.0
    } else {
        1.0
    }
}
fn map_on(_point: &SimplePoint) -> f64 {
    1.0
}
fn map_none(_point: &SimplePoint) -> f64 {
    0.0
}
fn map_off(_point: &SimplePoint) -> f64 {
    -1.0
}

// track progress
struct WavProgress {
    cur_time: f64,
    cur_sample: u64,
    time_per_sample: f64,
}

impl WavProgress {
    fn advance(&mut self, dt: f64) -> u64 {
        // this point can be drawn until this time
        let next_time = self.cur_time + dt;
        // current time in wav file (wav time advances in ticks of time_per_sample)
        let cur_sample_time = self.cur_sample as f64 * self.time_per_sample;
        // time range that is available to output samples for this point
        let sample_range = (next_time - cur_sample_time).max(0.0);
        // amount of samples that can be drawn for this point
        let n = (sample_range / self.time_per_sample).ceil() as u64;
        // advance
        self.cur_time = next_time;
        self.cur_sample = self.cur_sample + n;
        //        eprintln!(
        //            "next_time={}, cur_sample_time={}, sample_range={}, n={}, time_per_sample={}",
        //            next_time, cur_sample_time, sample_range, n, self.time_per_sample
        //        );
        n
    }
}

// struct that holds mapped points of a frame and the total traveled distance
// TODO: add angle info?
#[derive(Debug)]
struct FramePoints {
    pos: Vec<f64>,
    dist: f64,
}

fn main() {
    let mut options = get_options();

    let max_dist_per_frame = options.mdpm as f64 / options.fps as f64;

    let time_per_frame = 1.0 / options.fps as f64;
    let time_per_sample = 1.0 / options.sample_rate as f64;
    let time_per_point = 1.0 / options.pps;

    let mut cur_progress = WavProgress {
        cur_time: 0.0,
        cur_sample: 0,
        time_per_sample,
    };

    let animation: Box<Iterator<Item = Frame>> = if options.repeat {
        Box::new(Animation::stream(&mut options.input).memory_cycle())
    } else {
        Box::new(Animation::stream(&mut options.input))
    };

    let mut cur_pos = vec![0.0; options.channels.len()];

    for frame in animation {
        let mut points: Vec<FramePoints> = vec![];

        for point in frame.get_points() {
            let mut total_d = 0.0;

            let pos: Vec<f64> = options
                .channels
                .iter()
                .enumerate()
                .map(|(i, mc)| {
                    let next_pos = (mc.mapper)(point);

                    if mc.is_axis {
                        let d = points.last().map(|p| &p.pos).unwrap_or(&cur_pos)[i] - next_pos;
                        total_d += d * d
                    }

                    next_pos
                })
                .collect();

            points.push(FramePoints {
                pos,
                dist: total_d.sqrt(),
            });
        }

        let total_dist: f64 = points.iter().map(|p| p.dist).sum();

        // TODO: think about this more... -> make this editable
        // each point can use at least one sample time

        let guaranteed_per_sample = time_per_point * options.correctness;

        let guaranteed_time = guaranteed_per_sample * points.len() as f64;
        let shared_time = (time_per_frame - guaranteed_time).max(0.0);

        eprintln!(
            "guaranteed: {}, shared: {}, total_per_frame: {}, points: {}",
            guaranteed_time,
            shared_time,
            time_per_frame,
            points.len()
        );

        for point in points {
            // println!("{:?}", point);
            // moving to this point can use this amount of time of the shared_time
            let share_of_frame = point.dist / total_dist;
            let n = cur_progress.advance(shared_time * share_of_frame);

            // TODO: check max speed and adjust

            if n > 0 {
                // instruction for samples
                let steps: Vec<_> = options
                    .channels
                    .iter()
                    .enumerate()
                    .map(|(i, mc)| {
                        let next_pos = point.pos[i];
                        if mc.is_axis {
                            Step::Linear {
                                from: cur_pos[i],
                                step: (next_pos - cur_pos[i]) / n as f64,
                            }
                        } else {
                            Step::Jump(next_pos)
                        }
                    })
                    .collect();

                for i in 1..=n {
                    let samples: Vec<_> = steps
                        .iter()
                        .map(|step| match step {
                            Step::Linear { from, step } => (from + step * i as f64),
                            Step::Jump(pos) => *pos,
                        })
                        .collect();

                    options.output.write(&samples).unwrap();
                }
            }

            cur_pos = point.pos;

            let n = cur_progress.advance(guaranteed_per_sample);

            for _ in 1..=n {
                options.output.write(&cur_pos).unwrap();
            }
        }
    }

    options.output.finish().unwrap();
}

use byteorder::{LittleEndian, ReadBytesExt};
use clap::{App, Arg};
use hound::{Error as HoundError, WavReader};
use ilda::animation::{AnimationStreamWriter, Frame};
use ilda::{IldaError, SimplePoint};
use rustfft::algorithm::Radix4;
use rustfft::num_complex::Complex;
use rustfft::FFT;
use std::fs::File;
use std::io::{self, Error as IoError, ErrorKind, Read, Write};
use std::num::{ParseFloatError, ParseIntError};

#[derive(Debug)]
enum Error {
    IoError(IoError),
    FailedToInferInputFile,
    UnsupportedBitsPerSample,
    ParseFloatError(ParseFloatError),
    ParseIntError(ParseIntError),
    IldaError(IldaError),
    HoundError(HoundError),
}

impl From<ParseFloatError> for Error {
    fn from(error: ParseFloatError) -> Self {
        Error::ParseFloatError(error)
    }
}

impl From<ParseIntError> for Error {
    fn from(error: ParseIntError) -> Self {
        Error::ParseIntError(error)
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Error::IoError(error)
    }
}

impl From<IldaError> for Error {
    fn from(error: IldaError) -> Self {
        Error::IldaError(error)
    }
}

impl From<HoundError> for Error {
    fn from(error: HoundError) -> Self {
        Error::HoundError(error)
    }
}

enum BytesPerSample {
    OneByte,
    TwoBytes,
    FourBytes,
}

struct Options {
    input: Box<dyn Read>,
    output: Box<dyn Write>,
    raw_pcm: bool,
    fps: f64,
    bits_per_sample: u16,
    bins: u16,
    sample_rate: u32,
}

struct SamplesHoundReader {
    hound: WavReader<Box<dyn Read>>,
    sample_window: usize,
    sample_duration: usize,
}

struct SamplesRawReader {
    input: Box<dyn Read>,
    bps: BytesPerSample,
    channels: u16,
    sample_window: usize,
    sample_duration: usize,
}

impl Iterator for SamplesHoundReader {
    type Item = Result<Vec<Complex<f64>>, Error>;

    fn next(&mut self) -> Option<Result<Vec<Complex<f64>>, Error>> {
        let mut result = Vec::with_capacity(self.sample_window);

        let channels = self.hound.spec().channels as usize;

        let divisor = match self.hound.spec().bits_per_sample {
            8 => i8::max_value() as f64,
            16 => i16::max_value() as f64,
            32 => i32::max_value() as f64,
            _ => return Some(Err(Error::UnsupportedBitsPerSample)),
        };

        // collect samples
        for _ in 0..self.sample_window {
            let mut samples = Vec::with_capacity(channels as usize);
            for _ in 0..channels {
                let sample = match self.hound.samples::<i32>().next() {
                    Some(Err(e)) => return Some(Err(Error::HoundError(e))),
                    Some(Ok(sample)) => sample as f64 / divisor,
                    None => return None,
                };
                samples.push(sample);
            }
            let avg = samples.iter().sum::<f64>() / samples.len() as f64;
            result.push(Complex::new(avg, 0.0));
        }

        // discard the remaining samples
        for _ in 0..((self.sample_duration - self.sample_window) * channels) {
            self.hound.samples::<i32>().next();
        }

        Some(Ok(result))
    }
}

impl Iterator for SamplesRawReader {
    type Item = Result<Vec<Complex<f64>>, Error>;

    fn next(&mut self) -> Option<Result<Vec<Complex<f64>>, Error>> {
        let mut result = Vec::with_capacity(self.sample_window);

        // collect samples
        for _ in 0..self.sample_window {
            let mut samples = Vec::with_capacity(self.channels as usize);
            for _ in 0..self.channels {
                let sample = match self.bps {
                    BytesPerSample::OneByte => match self.input.read_i8() {
                        Ok(data) => data as f64 / i8::max_value() as f64,
                        Err(e) => match e.kind() {
                            ErrorKind::UnexpectedEof => return None,
                            _ => return Some(Err(Error::IoError(e))),
                        },
                    },
                    BytesPerSample::TwoBytes => match self.input.read_i16::<LittleEndian>() {
                        Ok(data) => data as f64 / i16::max_value() as f64,
                        Err(e) => match e.kind() {
                            ErrorKind::UnexpectedEof => return None,
                            _ => return Some(Err(Error::IoError(e))),
                        },
                    },
                    BytesPerSample::FourBytes => match self.input.read_i32::<LittleEndian>() {
                        Ok(data) => data as f64 / i32::max_value() as f64,
                        Err(e) => match e.kind() {
                            ErrorKind::UnexpectedEof => return None,
                            _ => return Some(Err(Error::IoError(e))),
                        },
                    },
                };
                samples.push(sample);
            }

            let avg = samples.iter().sum::<f64>() / samples.len() as f64;
            result.push(Complex::new(avg, 0.0));
        }

        // discard the remaining samples
        //        for _ in 0..((self.sample_duration - self.sample_window) * self.channels) {
        //            self.hound.samples::<i32>().next()
        //        }

        Some(Ok(result))
    }
}

fn get_options<'a>() -> Result<Options, Error> {
    let matches = App::new("wav2ilda")
        .version("0.1.0")
        .author("Lukas <lukasjapan@gmail.com>")
        .about("A sound visualizer tool that creates ilda animations from wav files.")
        .arg(
            Arg::with_name("RAW")
                .short("r")
                .long("raw")
                .help("Input data does not contain a wav header. (raw pcm samples)"),
        )
        .arg(
            Arg::with_name("FPS")
                .short("f")
                .long("fps")
                .default_value("20.0")
                .help("The number of frames per second.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("SAMPLERATE")
                .short("s")
                .long("sample-rate")
                .default_value("44100")
                .help("Sample rate of raw pcm data. This value is ignored unless the input is raw pcm.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("BPS")
                .short("b")
                .long("bps")
                .default_value("16")
                .help("Bits per sample of raw pcm. This value is ignored unless the input is raw pcm.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("BINS")
                .short("i")
                .long("bins")
                .default_value("64")
                .help("Amount of equalizer bins for the visualization. Higher values lead to more complex but more detailed output.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FILES")
                .multiple(true)
                .help("Specify 0~2 filenames.
0 filename: Read the input from STDIN and write the output to STDOUT
1 filename with .wav extension: Read the input from the given file and write the output to STDOUT
1 filename with .ild extension: Read the input from STDIN and write the output to the given file
2 filenames: Read the input from the first file and write the output to the second file")
                .max_values(2),
        )
        .get_matches();

    let raw_pcm = matches.is_present("RAW");

    let sample_rate: u32 = matches.value_of("SAMPLERATE").unwrap().parse()?;

    let bits_per_sample: u16 = matches.value_of("BPS").unwrap().parse()?;

    let bins: u16 = matches.value_of("BINS").unwrap().parse()?;

    let fps: f64 = matches.value_of("FPS").unwrap().parse()?;

    let files: Vec<&str> = match matches.values_of("FILES") {
        Some(files) => files.collect(),
        None => vec![],
    };

    let (file_in, file_out) = match files.len() {
        1 => match &files[0].to_lowercase()[files[0].len() - 4..] {
            ".wav" => (Some(files[0]), None),
            ".ild" => (None, Some(files[0])),
            _ => return Err(Error::FailedToInferInputFile),
        },
        2 => (Some(files[0]), Some(files[1])),
        _ => (None, None),
    };

    let input: Box<dyn Read> = match file_in {
        Some(filename) => Box::new(File::open(filename)?),
        None => Box::new(io::stdin()),
    };

    let output: Box<dyn Write> = match file_out {
        Some(filename) => Box::new(File::create(filename)?),
        None => Box::new(io::stdout()),
    };

    eprintln!("Input:           {}", file_in.unwrap_or("STDIN"));
    eprintln!("Output:          {}", file_out.unwrap_or("STDOUT"));

    Ok(Options {
        input,
        output,
        sample_rate,
        raw_pcm,
        bits_per_sample,
        bins,
        fps,
    })
}

struct FrequencyWaves {
    reverse: bool,
}

impl FrequencyWaves {
    fn new() -> FrequencyWaves {
        FrequencyWaves { reverse: false }
    }

    fn bins_to_frame(&mut self, bins: Vec<f64>) -> Frame {
        let len = bins.len() as f64;
        let points: Vec<_> = bins
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let x = -1.0 + (i as f64 + 0.5) as f64 * 2.0 / len;
                let y = if i % 2 == 0 { *v } else { -*v };
                SimplePoint {
                    x: (if self.reverse { x } else { -x } * i16::max_value() as f64) as i16,
                    y: (y * i16::max_value() as f64) as i16,
                    r: 255,
                    g: 255,
                    b: 255,
                    is_blank: false,
                }
            })
            .collect();

        self.reverse = !self.reverse;

        Frame::new(points, None, None)
    }
}

fn get_value(samples: &Vec<Complex<f64>>, from_index: f64, to_index: f64) -> f64 {
    let from_full = from_index.ceil();
    let to_full = to_index.floor();
    let first_fraction = from_full - from_index;
    let last_fraction = to_index - to_full;
    let from_full = from_full as usize;
    let to_full = to_full as usize;

    let mut sum = 0.0;
    sum = sum + first_fraction * samples[from_full - 1].norm_sqr().sqrt();
    if from_full < to_full {
        for i in from_full..to_full {
            sum = sum + samples[i].norm_sqr().sqrt();
        }
    } else {
        for i in to_full..from_full {
            sum = sum - samples[i].norm_sqr().sqrt();
        }
    }
    sum = sum + last_fraction * samples[to_full].norm_sqr().sqrt();
    (sum / (to_index - from_index))
}

fn main() -> Result<(), Error> {
    eprintln!("wav2ilda - https://github.com/lukasjapan/ilda-tools");
    eprintln!();
    let mut options = get_options()?;

    let sample_window = 256;
    let sample_duration = (options.sample_rate as f64 / options.fps) as usize;

    let reader: Box<dyn Iterator<Item = Result<Vec<Complex<f64>>, Error>>> = if options.raw_pcm {
        eprintln!("Raw PCM:       Yes");

        let bps = match options.bits_per_sample {
            8 => BytesPerSample::OneByte,
            16 => BytesPerSample::TwoBytes,
            32 => BytesPerSample::FourBytes,
            _ => return Err(Error::UnsupportedBitsPerSample),
        };

        Box::new(SamplesRawReader {
            input: options.input,
            bps,
            channels: 2,
            sample_window,
            sample_duration,
        })
    } else {
        eprintln!("Raw PCM:         No");

        let hound = WavReader::new(options.input)?;
        let divisor = match hound.spec().bits_per_sample {
            8 => i8::max_value() as f64,
            16 => i16::max_value() as f64,
            32 => i32::max_value() as f64,
            _ => return Err(Error::UnsupportedBitsPerSample),
        };
        let reader = Box::new(SamplesHoundReader {
            hound,
            sample_window,
            sample_duration,
        });

        options.sample_rate = reader.hound.spec().sample_rate;
        options.bits_per_sample = reader.hound.spec().bits_per_sample;

        reader
    };

    eprintln!("Sample rate:     {}", options.sample_rate);
    eprintln!("Bits per sample: {}", options.bits_per_sample);

    let fft = Radix4::new(sample_window, false);
    let mut output = vec![Complex::new(0.0, 0.0); sample_window];

    let mut writer = AnimationStreamWriter::new(options.output);

    // display 20 - 20kHz range
    let from_index = (sample_window as f64 * 100.0 / options.sample_rate as f64).max(1.0);
    let to_index = sample_window as f64 * (20000.0 / options.sample_rate as f64).min(0.5);

    let log_space_from = from_index.log2();
    let log_space_step = (to_index.log2() - log_space_from) / options.bins as f64;

    let mut vis = FrequencyWaves::new();

    for result in reader {
        let mut samples = result?;
        fft.process(&mut samples, &mut output);

        let bins: Vec<_> = (0..options.bins)
            .map(|i| {
                let from_index = (2.0 as f64).powf(log_space_from + i as f64 * log_space_step);
                let to_index =
                    (2.0 as f64).powf(log_space_from + (i as f64 + 1.0) * log_space_step);
                get_value(&samples, from_index, to_index)
            })
            .collect();

        writer.write_frame(&vis.bins_to_frame(bins))?;
    }

    writer.finalize()?;

    Ok(())
}

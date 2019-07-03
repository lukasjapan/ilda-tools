use byteorder::{LittleEndian, ReadBytesExt};
use clap::{App, Arg};
use hound::{Error as HoundError, WavReader};
use ilda::animation::{AnimationStreamWriter, Frame};
use ilda::{IldaError, SimplePoint};
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
    InvalidChannel(char),
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

struct SimplePointRawReader {
    input: Box<dyn Read>,
    bps: BytesPerSample,
    mapping_conf: String,
}

fn to_point(normalized_input: Vec<f64>, mapping_conf: &str) -> Result<SimplePoint, Error> {
    let has_color =
        mapping_conf.contains('r') || mapping_conf.contains('g') || mapping_conf.contains('b');

    let mut result = SimplePoint {
        x: 0,
        y: 0,
        r: if has_color { 0 } else { 255 },
        g: if has_color { 0 } else { 255 },
        b: if has_color { 0 } else { 255 },
        is_blank: false,
    };

    for (i, c) in mapping_conf.chars().enumerate() {
        match c {
            'x' => result.x = (normalized_input[i] * i16::max_value() as f64) as i16,
            'X' => result.x = (-normalized_input[i] * i16::max_value() as f64) as i16,
            'y' => result.y = (normalized_input[i] * i16::max_value() as f64) as i16,
            'Y' => result.y = (-normalized_input[i] * i16::max_value() as f64) as i16,
            'r' => result.r = ((normalized_input[i] + 1.0) / 2.0 * u8::max_value() as f64) as u8,
            'g' => result.g = ((normalized_input[i] + 1.0) / 2.0 * u8::max_value() as f64) as u8,
            'b' => result.b = ((normalized_input[i] + 1.0) / 2.0 * u8::max_value() as f64) as u8,
            'l' => result.is_blank = normalized_input[i] < 0.0,
            '_' => {}
            _ => return Err(Error::InvalidChannel(c)),
        }
    }

    Ok(result)
}

impl Iterator for SimplePointRawReader {
    type Item = Result<SimplePoint, Error>;

    fn next(&mut self) -> Option<Result<SimplePoint, Error>> {
        let mut normalized_input: Vec<_> = vec![];

        for _ in 0..self.mapping_conf.len() {
            let value = match self.bps {
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

            normalized_input.push(value);
        }

        Some(to_point(normalized_input, &self.mapping_conf))
    }
}

struct SimplePointHoundReader {
    hound: WavReader<Box<Read>>,
    mapping_conf: String,
}

impl Iterator for SimplePointHoundReader {
    type Item = Result<SimplePoint, Error>;

    fn next(&mut self) -> Option<Result<SimplePoint, Error>> {
        let mut normalized_input: Vec<_> = vec![];

        for _ in 0..self.mapping_conf.len() {
            let value = match self.hound.samples::<i32>().next() {
                Some(Err(e)) => return Some(Err(Error::HoundError(e))),
                Some(Ok(sample)) => match self.hound.spec().bits_per_sample {
                    8 => sample as f64 / i8::max_value() as f64,
                    16 => sample as f64 / i16::max_value() as f64,
                    32 => sample as f64 / i32::max_value() as f64,
                    _ => return Some(Err(Error::UnsupportedBitsPerSample)),
                },
                None => return None,
            };
            normalized_input.push(value);
        }

        Some(to_point(normalized_input, &self.mapping_conf))
    }
}

struct Options {
    input: Box<dyn Read>,
    output: Box<dyn Write>,
    raw_pcm: bool,
    fps: f64,
    bits_per_sample: u32,
    sample_rate: u32,
    mapping_conf: String,
}

fn get_options<'a>() -> Result<Options, Error> {
    let matches = App::new("ildawav2ilda")
        .version("0.1.0")
        .author("Lukas <lukasjapan@gmail.com>")
        .about("Creates an ilda file from a wav file that contains laser projector control signals. (e.g. files that have been created with the ilda2wav tool.")
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
                .help("The number of frames per second. This will determine the amount of samples that go into one frame.")
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
            Arg::with_name("CHANNELS")
                .help("A string that defines the channel configuration of the file. Use one or more of the following characters:
x: X-Axis
X: X-Axis mirrored
y: Y-Axis
Y: Y-Axis mirrored
r: Intensity of Red component
g: Intensity of Green component
b: Intensity of Blue component
l: Blanking signal
_: Ignore 

The channel count must match the input file channel count.

Ex:
A stereo file that controls the axis only: xy
A 5.1 channel file that controls the axis with rear channels and includes the blanking signal: __l_xy
")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("FILES")
                .multiple(true)
                .help("Specify 0~2 filenames.
0 filename: Read the input from STDIN and write the output to STDOUT
1 filename with .wav extension: Read the input from the given file and write the output to STDOUT
1 filename with .ild extension: Read the input from STDIN and write the output to the given file
2 filenames: Read the input from the first file and write the output to the second file")
                .max_values(2)
                .index(2),
        )
        .get_matches();

    let raw_pcm = matches.is_present("RAW");

    let sample_rate: u32 = matches.value_of("SAMPLERATE").unwrap().parse()?;

    let bits_per_sample: u32 = matches.value_of("BPS").unwrap().parse()?;

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

    let mapping_conf = matches.value_of("CHANNELS").unwrap().to_string();

    eprintln!("Input:           {}", file_in.unwrap_or("STDIN"));
    eprintln!("Output:          {}", file_out.unwrap_or("STDOUT"));
    eprintln!("Mapping:         {}", mapping_conf);

    Ok(Options {
        input,
        output,
        sample_rate,
        raw_pcm,
        bits_per_sample,
        mapping_conf,
        fps,
    })
}

fn main() -> Result<(), Error> {
    eprintln!("ildawav2ilda - https://github.com/lukasjapan/ilda-tools");
    eprintln!();

    let options = get_options()?;

    let reader: Box<dyn Iterator<Item = Result<SimplePoint, Error>>> = if options.raw_pcm {
        eprintln!(
            "Raw PCM:       Yes - {}bit @ {}Hz",
            options.bits_per_sample, options.sample_rate
        );

        let bps = match options.bits_per_sample {
            8 => BytesPerSample::OneByte,
            16 => BytesPerSample::TwoBytes,
            32 => BytesPerSample::FourBytes,
            _ => return Err(Error::UnsupportedBitsPerSample),
        };

        Box::new(SimplePointRawReader {
            input: options.input,
            bps,
            mapping_conf: options.mapping_conf,
        })
    } else {
        eprintln!("Raw PCM:         No");

        let reader = Box::new(SimplePointHoundReader {
            hound: WavReader::new(options.input)?,
            mapping_conf: options.mapping_conf,
        });

        eprintln!("Sample rate:     {}", reader.hound.spec().sample_rate);
        eprintln!("Bits per sample: {}", reader.hound.spec().bits_per_sample);

        reader
    };

    let mut writer = AnimationStreamWriter::new(options.output);

    let mut current_time = 0.0;
    let time_per_sample = 1.0 / options.sample_rate as f64;
    let time_per_frame = 1.0 / options.fps;
    let mut next_frame = time_per_frame;

    let mut points: Vec<SimplePoint> = vec![];
    for result in reader {
        points.push(result?);
        current_time = current_time + time_per_sample;
        if current_time > next_frame {
            writer.write_frame(&Frame::new(
                points.clone(),
                Some(String::from("")),
                Some(String::from("")),
            ))?;
            points.clear();
            next_frame = next_frame + time_per_frame;
        }
    }

    if !points.is_empty() {
        writer.write_frame(&Frame::new(
            points.clone(),
            Some(String::from("")),
            Some(String::from("")),
        ))?;
    }

    writer.finalize()?;

    Ok(())
}

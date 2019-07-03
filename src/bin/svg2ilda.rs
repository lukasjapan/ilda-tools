use chrono::Local;
use clap::{App, Arg};
use ilda::SimplePoint;
use ilda::animation::{Frame, Animation};
use ilda::writer::IldaWriter;
use lyon_geom::cubic_bezier::Flattened;
use lyon_geom::euclid::Point2D;
use lyon_geom::CubicBezierSegment;
use std::fs::File;
use std::io::{self, Error as IoError, Read, Write};
use std::num::ParseFloatError;
use usvg::{
    Color, Error as UsvgError, Fill, NodeKind, Paint, Path, PathSegment, Stroke, Transform, Tree,
    Visibility,
};

struct Point {
    x: f64,
    y: f64,
    color: Color,
    blank: bool,
}

struct Options {
    input: Box<dyn Read>,
    output: Box<dyn Write>,
    name: String,
    company_name: String,
    invert: bool,
    tolerance: f64,
}

const DEFAULT_POINT: Point = Point {
    x: 0.0,
    y: 0.0,
    color: Color {
        red: 255,
        green: 255,
        blue: 255,
    },
    blank: true,
};

#[derive(Debug)]
enum Error {
    UsvgError(UsvgError),
    ParseFloatError(ParseFloatError),
    IoError(IoError),
    FailedToInferInputFile,
    InvalidSvg,
    SvgTooComplexForIlda,
}

impl From<UsvgError> for Error {
    fn from(error: UsvgError) -> Self {
        Error::UsvgError(error)
    }
}

impl From<ParseFloatError> for Error {
    fn from(error: ParseFloatError) -> Self {
        Error::ParseFloatError(error)
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Error::IoError(error)
    }
}

fn get_options<'a>() -> Result<Options, Error> {
    let matches = App::new("svg2ilda")
        .version("0.1.0")
        .author("Lukas <lukasjapan@gmail.com>")
        .about("Generates an ILDA file from svg.")
        .arg(
            Arg::with_name("TOLERANCE")
                .required(false)
                .short("t")
                .long("tolerance")
                .default_value("0.1")
                .help("Tolerance when plotting curves. Lower values (above 0) will produce smoother curves.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("NAME")
                .required(false)
                .short("n")
                .long("name")
                .help("The name to write into the ILDA header. If not given, the filename is used.\
If the input comes from STDIN, 's_YYMMDD' is used with YYMMDD being substituted by the current date.\
Please note that the name in the header can only hold 8 bytes and will be cut if it is longer.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("COMPANY_NAME")
                .required(false)
                .short("c")
                .long("company")
                .help("The company name to write into the ILDA header. If not given, 'svg2ilda' will be used.\
Please note that the company name in the header can only hold 8 bytes and will be cut if it is longer.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("INVERT")
                .short("i")
                .long("invert")
                .help("Inverts colors. Particular usefull with black and white files."),
        )
        .arg(
            Arg::with_name("FILES")
                .multiple(true)
                .required(false)
                .help(
                    r#"Specify 0~2 filenames.
0 filename: Read the input from STDIN and write the output to STDOUT
1 filename with .svg extension: Read the input from the given file and write the output to STDOUT
1 filename with .ild extension: Read the input from STDIN and write the output to the given file
2 filenames: Read the input from the first file and write the output to the second file
                "#,
                )
                .max_values(2)
                .index(1),
        )
        .get_matches();

    let files: Vec<&str> = match matches.values_of("FILES") {
        Some(files) => files.collect(),
        None => vec![],
    };

    let (file_in, file_out) = match files.len() {
        1 => match &files[0].to_lowercase()[files[0].len() - 4..] {
            ".ild" => (None, Some(files[0])),
            ".svg" => (Some(files[0]), None),
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

    let name = matches
        .value_of("NAME")
        .map_or(format!("s_{}", Local::now().format("%y%m%d")), String::from);

    let company_name = String::from(matches.value_of("COMPANY_NAME").unwrap_or("svg2ilda"));

    let invert = matches.is_present("INVERT");

    let tolerance = matches.value_of("TOLERANCE").unwrap().parse()?;

    eprintln!("Input:         {}", file_in.unwrap_or("STDIN"));
    eprintln!("Output:        {}", file_in.unwrap_or("STDOUT"));
    eprintln!("Name:          {} / {}", &name[0..8], &company_name[0..8]);
    eprintln!("Invert colors: {}", if invert { "Yes" } else { "No" });
    eprintln!("Tolerance:     {}", tolerance);

    Ok(Options {
        input,
        output,
        name,
        invert,
        company_name,
        tolerance,
    })
}

fn collect_points_from_node(
    node: &usvg::Node,
    points: &mut Vec<Point>,
    transform: &Transform,
    options: &Options,
) {
    match &*node.borrow() {
        NodeKind::Svg(_) => {
            for child in node.children() {
                collect_points_from_node(&child, points, transform, options);
            }
        }
        NodeKind::Path(path) => {
            if path.visibility != Visibility::Visible {
                return;
            }

            let mut path_transform = transform.clone();
            path_transform.append(&path.transform);
            let path_transform = path_transform;

            let mut color = if let Path {
                stroke:
                    Some(Stroke {
                        paint: Paint::Color(color),
                        ..
                    }),
                ..
            } = path
            {
                *color
            } else if let Path {
                fill:
                    Some(Fill {
                        paint: Paint::Color(color),
                        ..
                    }),
                ..
            } = path
            {
                *color
            } else {
                Color::white()
            };

            if options.invert {
                color = Color {
                    red: 255 - color.red,
                    green: 255 - color.green,
                    blue: 255 - color.blue,
                }
            }

            let mut first_index = points.len();
            for segment in &path.segments {
                match segment {
                    PathSegment::MoveTo { x, y } => {
                        let coord = path_transform.apply(*x, *y);
                        points.push(Point {
                            x: coord.0,
                            y: coord.1,
                            color,
                            blank: true,
                        })
                    }
                    PathSegment::LineTo { x, y } => {
                        let coord = path_transform.apply(*x, *y);
                        points.push(Point {
                            x: coord.0,
                            y: coord.1,
                            color,
                            blank: false,
                        })
                    }
                    PathSegment::CurveTo {
                        x,
                        y,
                        x1,
                        y1,
                        x2,
                        y2,
                    } => {
                        let last = points.last().unwrap_or(&DEFAULT_POINT);
                        let coord = path_transform.apply(*x, *y);
                        let coord1 = path_transform.apply(*x1, *y1);
                        let coord2 = path_transform.apply(*x2, *y2);

                        let bezier = CubicBezierSegment {
                            from: Point2D::new(last.x, last.y),
                            to: Point2D::new(coord.0, coord.1),
                            ctrl1: Point2D::new(coord1.0, coord1.1),
                            ctrl2: Point2D::new(coord2.0, coord2.1),
                        };

                        for point in Flattened::new(bezier, options.tolerance) {
                            points.push(Point {
                                x: point.x,
                                y: point.y,
                                color,
                                blank: false,
                            })
                        }
                    }
                    PathSegment::ClosePath => {
                        let first_in_path = points.get(first_index).unwrap_or(&DEFAULT_POINT);
                        let coord = path_transform.apply(first_in_path.x, first_in_path.y);
                        points.push(Point {
                            x: coord.0,
                            y: coord.1,
                            color,
                            blank: false,
                        });
                        first_index = points.len();
                    }
                }
            }

            for child in node.children() {
                collect_points_from_node(&child, points, &path_transform, options);
            }
        }
        NodeKind::Group(group) => {
            let mut group_transform = transform.clone();
            group_transform.append(&group.transform);

            for child in node.children() {
                collect_points_from_node(&child, points, &group_transform, options);
            }
        }
        _ => {} // other elements not supported
    }
}

fn main() -> Result<(), Error> {
    eprintln!("svg2ilda - https://github.com/lukasjapan/ilda-tools");
    eprintln!();

    let mut options = get_options()?;

    let mut data: Vec<u8> = vec![];
    options.input.read_to_end(&mut data)?;

    let tree = Tree::from_data(&data, &usvg::Options::default())?;
    let root = tree.root();

    let view_box = match &*root.borrow() {
        NodeKind::Svg(svg) => svg.view_box,
        _ => return Err(Error::InvalidSvg), // This should never happen
    };

    let mut points: Vec<Point> = vec![];
    collect_points_from_node(&root, &mut points, &Transform::default(), &options);

    // Build a matrix that transform to ILDA coordinates
    let dx = -view_box.rect.x - view_box.rect.width / 2.0;
    let dy = -view_box.rect.y - view_box.rect.height / 2.0;
    let s = i16::max_value() as f64 / view_box.rect.width.max(view_box.rect.height) * 2.0;
    let mut t = Transform::default();
    t.append(&mut Transform::new_scale(s, -s));
    t.append(&mut Transform::new_translate(dx, dy));

    // do the actual transformation and filter out values that are outside the viewbox
    let mut blank_next = false;
    let points: Vec<_> = points
        .into_iter()
        .filter_map(|point| {
            let (x, y) = t.apply(point.x, point.y);
            // out of bound
            if x.round() < i16::min_value() as f64
                || x.round() > i16::max_value() as f64
                || y.round() < i16::min_value() as f64
                || y.round() > i16::max_value() as f64
            {
                blank_next = true;
                None
            } else {
                Some(SimplePoint {
                    x: x.round() as i16,
                    y: y.round() as i16,
                    r: point.color.red,
                    g: point.color.green,
                    b: point.color.green,
                    is_blank: if blank_next {
                        blank_next = false;
                        true
                    } else {
                        point.blank
                    },
                })
            }
        })
        .collect();

    let len = points.len();
    if len > u16::max_value() as usize {
        return Err(Error::SvgTooComplexForIlda);
    }

    eprintln!("Points:        {}", len);

    let frame = Frame::new(points, Some(options.name), Some(options.company_name));
    let frames = vec![frame];
    let animation = Animation::new(frames);

    animation.write(options.output);

    Ok(())
}

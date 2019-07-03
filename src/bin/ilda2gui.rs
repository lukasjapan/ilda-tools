mod common;

use clap::{App, Arg};
use common::memory_cycle::MemoryCycleIteratorExt;
use common::timed_iterator::{TimedExt, TimedIteratorStrategy};
use glium::glutin::dpi::LogicalSize;
use glium::glutin::{ContextBuilder, ControlFlow, Event, EventsLoop, WindowBuilder, WindowEvent};
use glium::index::{NoIndices, PrimitiveType};
use glium::uniforms::EmptyUniforms;
use glium::{Display, DrawParameters, PolygonMode, Program, Surface, VertexBuffer};
use ilda::animation::{Animation, Frame};
use ilda::IldaError;
use std::fs::File;
use std::io::{self, Read, Error as IoError};
use std::num::{ParseFloatError, ParseIntError};

#[derive(Debug)]
enum Error {
    IoError(IoError),
    ParseFloatError(ParseFloatError),
    ParseIntError(ParseIntError),
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

struct OpenGLWindow {
    events_loop: EventsLoop,
    display: Display,
    program: Program,
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}
glium::implement_vertex!(Vertex, position, color);

impl OpenGLWindow {
    fn new(dimensions: LogicalSize) -> Self {
        let events_loop = EventsLoop::new();
        let wb = WindowBuilder::new()
            .with_title("ilda2gui")
            .with_dimensions(dimensions);
        let cb = ContextBuilder::new();
        let display = Display::new(wb, cb, &events_loop).unwrap();
        let vertex_shader = r#"
            #version 140
            in vec2 position;
            in vec4 color;
            flat out vec4 color_v;
            void main() {
                color_v = color;
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "#;
        let fragment_shader = r#"
            #version 140
            flat in vec4 color_v;
            out vec4 color_out;
            void main() {
                if (color_v.a < 0.5) { discard; }
                color_out = color_v;
            }
        "#;
        let program = Program::from_source(&display, vertex_shader, fragment_shader, None).unwrap();

        return OpenGLWindow {
            events_loop,
            display,
            program,
        };
    }

    fn draw(&self, frame: &Frame) {
        let points: Vec<Vertex> = frame
            .get_points()
            .iter()
            .map(|p| Vertex {
                position: [
                    0.99 * p.x as f32 / i16::max_value() as f32,
                    0.99 * p.y as f32 / i16::max_value() as f32,
                ],
                color: [
                    p.r as f32 / u8::max_value() as f32,
                    p.g as f32 / u8::max_value() as f32,
                    p.b as f32 / u8::max_value() as f32,
                    if p.is_blank { 0.0 } else { 1.0 },
                ],
            })
            .collect();

        let vertex_buffer = VertexBuffer::new(&self.display, &points).unwrap();
        let indices = NoIndices(PrimitiveType::LineStrip);

        let mut target = self.display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target
            .draw(
                &vertex_buffer,
                &indices,
                &self.program,
                &EmptyUniforms,
                &DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    ..Default::default()
                },
            )
            .unwrap();
        target.finish().unwrap();
    }

    fn process_events(&mut self) -> ControlFlow {
        let mut quit = false;

        self.events_loop.poll_events(|event| {
            if let Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } = event
            {
                quit = true;
            }
        });

        if quit {
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
    }
}

struct Options {
    input: Box<dyn Read>,
    strategy: TimedIteratorStrategy,
    repeat: bool,
    size: i32,
    fps: f32,
}

fn get_options<'a>() -> Result<Options, Error> {
    let matches = App::new("ilda2gui")
        .version("0.1.0")
        .author("Lukas <lukasjapan@gmail.com>")
        .about("Displays ILDA data in an OpenGL window.")
        .arg(
            Arg::with_name("SIZE")
                .short("s")
                .long("size")
                .default_value("800")
                .help("Sets the width and height of the window.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FPS")
                .short("f")
                .long("fps")
                .default_value("20.0")
                .help("The number of frames per second for this animation.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FILE")
                .help("Read data from this file. If not given, use STDIN instead.")
                .index(1),
        )
        .arg(
            Arg::with_name("REPEAT")
                .short("r")
                .long("repeat")
                .help("Plays the given ILDA data in an infinite loop."),
        )
        .arg(
            Arg::with_name("NOSLEEP")
                .short("n")
                .long("nosleep")
                .help("Does not sleep between rendering frames. Only needed if the window should stay reactive with very low fps."),
        )
        .get_matches();

    let input: Box<dyn Read> = match matches.value_of("FILE") {
        Some(filename) => Box::new(File::open(filename)?),
        None => Box::new(io::stdin()),
    };

    let repeat = matches.is_present("REPEAT");

    let strategy = if matches.is_present("NOSLEEP") {
        TimedIteratorStrategy::Repeat
    } else {
        TimedIteratorStrategy::Sleep
    };

    let size = matches.value_of("SIZE").unwrap().parse()?;

    let fps = matches.value_of("FPS").unwrap().parse()?;

    Ok(Options {
        input,
        repeat,
        strategy,
        size,
        fps,
    })
}

fn main() -> Result<(), Error> {
    let mut options = get_options()?;

    let animation_stream = Animation::stream(&mut options.input);

    let mut window = OpenGLWindow::new(LogicalSize {
        width: options.size as f64,
        height: options.size as f64,
    });

    let mut iter: Box<Iterator<Item = Frame>> = Box::new(animation_stream);

    if options.repeat {
        iter = Box::new(iter.memory_cycle());
    }

    let iter = iter.timed(options.fps, options.strategy);

    for frame in iter {
        if let ControlFlow::Break = window.process_events() {
            break;
        }

        window.draw(&frame);
    }

    Ok(())
}

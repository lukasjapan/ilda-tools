# ilda-tools

Several command line tools to play/process/stream/... ILDA files.

## Install

Install the Rust compiler by following instructions on [rustup.rs](https://rustup.rs/).

Compile binaries with

```bash
cargo build --release
```

Binaries are compiled into the `target/release` directory.

## Tools

### ilda2gui

```
Displays ILDA data in an OpenGL window.

USAGE:
    ilda2gui [FLAGS] [OPTIONS] [FILE]

FLAGS:
    -n, --nosleep    Does not sleep between rendering frames. Only needed if the window should stay reactive with very
                     low fps.
    -r, --repeat     Plays the given ILDA data in an infinite loop.
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -f, --fps <FPS>      The number of frames per second for this animation. [default: 20.0]
    -s, --size <SIZE>    Sets the width and height of the window. [default: 800]

ARGS:
    <FILE>    Read data from this file. If not given, use STDIN instead.
```

### ilda2wav

```
Generates a wav file for an ILDA projector hooked to a sound card.

USAGE:
    ilda2wav [FLAGS] [OPTIONS] <CHANNELS> [FILES]...

FLAGS:
    -a, --raw        Output raw PCM data. (Do not write wav header)
    -r, --repeat     Repeats the input animation forever. Can only be used if outputting raw PCM samples to STDOUT.
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --bps <BPS>                    Bits per sample of the output wav. [default: 16]
    -c, --correctness <CORRECTNESS>    Defines how much time should be used as a minimum per point.
                                       0~1: points may be dropped
                                       1: Guarantee at least pps points per second (default)
                                       1~: Use extra time per point
                                       
                                       A value above 1 may lower the pps of your device on frames with a lot of points.
                                       For example a value of 2 will allocate the double amount of time per point,
                                       effectively cutting pps in half.
                                       
                                       If a frame contains too many points for the projector to handle, a value below 1
                                       allows points to be dropped from rendering.
                                       Points that are close to each other are more likely to be dropped.
                                       Any value above zero may slow down the animation. [default: 1.0]
    -f, --fps <FPS>                    Try to draw this number of frames per second. [default: 20.0]
    -m, --mdps <MDPS>                  Meh - Need to think about how to implement this constraint. This should probably
                                       be related to the pps setting [default: 100]
    -p, --pps <PPS>                    Point per second of the projector. The maximum limit of points that is sent to
                                       the projector per second. [default: 10000]
    -s, --sample-rate <SAMPLERATE>     Sample rate of the output wav. [default: 44100]

ARGS:
    <CHANNELS>    A string that defines the output channel configuration. Use one or more of the following
                  characters:
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
                  A 5.1 channel file that controls the axis with rear channels and includes the blanking signal:
                  __l_xy
    <FILES>...    Specify 0~2 filenames.
                  0 filename: Read the input from STDIN and write the output to STDOUT
                  1 filename with .ild extension: Read the input from the given file and write the output to STDOUT
                  1 filename with .wav extension: Read the input from STDIN and write the output to the given file
                  2 filenames: Read the input from the first file and write the output to the second file
                  
                  Warning: If writing to STDOUT, the output file will be buffered unless raw PCM samples are
                  requested.
```

### ildawav2ilda

```
Creates an ilda file from a wav file that contains laser projector control signals. (e.g. files that have been created
with the ilda2wav tool.

USAGE:
    ildawav2ilda [FLAGS] [OPTIONS] <CHANNELS> [FILES]...

FLAGS:
    -r, --raw        Input data does not contain a wav header. (raw pcm samples)
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --bps <BPS>                   Bits per sample of raw pcm. This value is ignored unless the input is raw pcm.
                                      [default: 16]
    -f, --fps <FPS>                   The number of frames per second. This will determine the amount of samples that go
                                      into one frame. [default: 20.0]
    -s, --sample-rate <SAMPLERATE>    Sample rate of raw pcm data. This value is ignored unless the input is raw pcm.
                                      [default: 44100]

ARGS:
    <CHANNELS>    A string that defines the channel configuration of the file. Use one or more of the following
                  characters:
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
                  A 5.1 channel file that controls the axis with rear channels and includes the blanking signal:
                  __l_xy
    <FILES>...    Specify 0~2 filenames.
                  0 filename: Read the input from STDIN and write the output to STDOUT
                  1 filename with .wav extension: Read the input from the given file and write the output to STDOUT
                  1 filename with .ild extension: Read the input from STDIN and write the output to the given file
                  2 filenames: Read the input from the first file and write the output to the second file
```

### svg2ilda

```
Generates an ILDA file from svg.

USAGE:
    svg2ilda [FLAGS] [OPTIONS] [FILES]...

FLAGS:
    -i, --invert     Inverts colors. Particular usefull with black and white files.
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --company <COMPANY_NAME>    The company name to write into the ILDA header. If not given, 'svg2ilda' will be
                                    used.Please note that the company name in the header can only hold 8 bytes and will
                                    be cut if it is longer.
    -n, --name <NAME>               The name to write into the ILDA header. If not given, the filename is used.If the
                                    input comes from STDIN, 's_YYMMDD' is used with YYMMDD being substituted by the
                                    current date.Please note that the name in the header can only hold 8 bytes and will
                                    be cut if it is longer.
    -t, --tolerance <TOLERANCE>     Tolerance when plotting curves. Lower values (above 0) will produce smoother curves.
                                    [default: 0.1]

ARGS:
    <FILES>...    Specify 0~2 filenames.
                  0 filename: Read the input from STDIN and write the output to STDOUT
                  1 filename with .svg extension: Read the input from the given file and write the output to STDOUT
                  1 filename with .ild extension: Read the input from STDIN and write the output to the given file
                  2 filenames: Read the input from the first file and write the output to the second file
```

### wav2ilda

```
A sound visualizer tool that creates ilda animations from wav files.

USAGE:
    wav2ilda [FLAGS] [OPTIONS] [FILES]...

FLAGS:
    -r, --raw        Input data does not contain a wav header. (raw pcm samples)
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i, --bins <BINS>                 Amount of equalizer bins for the visualization. Higher values lead to more complex
                                      but more detailed output. [default: 64]
    -b, --bps <BPS>                   Bits per sample of raw pcm. This value is ignored unless the input is raw pcm.
                                      [default: 16]
    -f, --fps <FPS>                   The number of frames per second. [default: 20.0]
    -s, --sample-rate <SAMPLERATE>    Sample rate of raw pcm data. This value is ignored unless the input is raw pcm.
                                      [default: 44100]

ARGS:
    <FILES>...    Specify 0~2 filenames.
                  0 filename: Read the input from STDIN and write the output to STDOUT
                  1 filename with .wav extension: Read the input from the given file and write the output to STDOUT
                  1 filename with .ild extension: Read the input from STDIN and write the output to the given file
                  2 filenames: Read the input from the first file and write the output to the second file
```
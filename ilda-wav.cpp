#include <iostream>
#include <fstream>
#include <boost/program_options.hpp>
#include "src/ilda_istream.hpp"
#include "src/ilda_wav.hpp"

namespace po = boost::program_options;

int main(int argc, char **argv)
{
    int fps;
    int rate;
    int pps;
    std::string signals;
    std::string invert;
    std::string output_filename;
    std::vector<std::string> filenames;

    po::options_description desc("ILDA-WAV converter\n"
                                 "\n"
                                 "Converts an .ild file to a .wav file.\n"
                                 "This is useful if you hook your galvometer and laser on a soundcard.\n"
                                 "Samples will be written with 2 byte signed integers per channel in little endian. (s16le)"
                                 "\n"
                                 "Usage: ilda-wav [options] [filename]\n"
                                 "If no filename is given ILDA-WAV will attempt to read from stdin.\n"
                                 "\n"
                                 "Allowed options:");
    desc.add_options()("fps,f", po::value<int>(&fps)->default_value(20), "Play speed in frames per second.");
    desc.add_options()("signals,s", po::value<std::string>(&signals)->default_value("xyl"), "Signals to include in the wav file. (ex: xyl)\n"
                                                                                            "Available signals:\n"
                                                                                            "x: (X-Axis)\n"
                                                                                            "y: (Y-Axis)\n"
                                                                                            "z: (Z-Axis)\n"
                                                                                            "l: (Laser blanking)\n"
                                                                                            "r: (Red)\n"
                                                                                            "g: (Green)\n"
                                                                                            "b: (Blue)");
    desc.add_options()("invert,i", po::value<std::string>(&invert), "Invert the given channels\n");
    desc.add_options()("rate,r", po::value<int>(&rate)->default_value(44100), "Sample rate.");
    desc.add_options()("pps,p", po::value<int>(&pps)->default_value(20000), "The number of points per second your galvo can handle. (Points will be dropped if there are too many)");
    desc.add_options()("output,o", po::value<std::string>(&output_filename), "Output file. If no filename is given ILDA-WAV will attempt to write to stdout.");
    desc.add_options()("help", "Display this help.");

    po::options_description o("");
    o.add(desc);
    o.add_options()("filename", po::value<std::vector<std::string>>(&filenames));
    po::positional_options_description p;
    p.add("filename", 1);

    po::variables_map vm;
    po::store(po::command_line_parser(argc, argv).options(o).positional(p).run(), vm);
    po::notify(vm);

    if (vm.count("help"))
    {
        cerr << o << "\n";
        return 1;
    }

    ifstream fin;
    ofstream fon;
    ostream *out = nullptr;
    istream *in = nullptr;

    if (filenames.size() > 0)
    {
        fin.open(filenames[0]);
        if (!fin)
        {
            throw runtime_error("File not found.");
        }

        in = &fin;
    }
    else
    {
        in = &cin;
    }

    if (vm.count("output"))
    {
        fon.open(output_filename);
        out = &fon;
    }
    else
    {
        out = &cout;
    }

    ILDAIStream ilda_input(*in);
    ILDAWavOutput wav_output(ilda_input, *out, fps, signals, invert, rate, pps);

    return wav_output.run();
}
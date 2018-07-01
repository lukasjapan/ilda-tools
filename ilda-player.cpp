#include <iostream>
#include <fstream>
#include <boost/program_options.hpp>
#include "src/ilda_istream.hpp"
#include "src/ilda_repeater.hpp"
#include "src/fltk_output.hpp"

namespace po = boost::program_options;

int main(int argc, char **argv)
{
    try
    {
        int fps;
        int width;
        int height;
        bool repeat;
        std::vector<std::string> filenames;

        po::options_description desc("ILDA-Player\n"
                                     "\n"
                                     "Plays .ild files in an fltk window.\n"
                                     "\n"
                                     "Usage: ilda-player [options] [filename]\n"
                                     "If no filename is given ILDA-Player will attempt to read the from stdin.\n"
                                     "\n"
                                     "Allowed options:");
        desc.add_options()("fps,f", po::value<int>(&fps)->default_value(20), "Frames per second.");
        desc.add_options()("width,w", po::value<int>(&width)->default_value(500), "Window width in pixel.");
        desc.add_options()("height,h", po::value<int>(&height)->default_value(500), "Window height in pixel.");
        desc.add_options()("repeat,r", po::bool_switch(&repeat), "Endlessly repeat the input.");
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
            cout << o << "\n";
            return 1;
        }

        ifstream f;
        istream *in = nullptr;

        if (filenames.size() > 0)
        {
            f.open(filenames[0]);
            if (!f)
            {
                throw runtime_error("File not found.");
            }

            in = &f;
        }
        else
        {
            in = &cin;
        }

        ILDAIStream ilda_in(*in);

        if (width <= 0)
            throw runtime_error("Width must be positive.");
        if (height <= 0)
            throw runtime_error("Height must be positive.");
        if (fps <= 0)
            throw runtime_error("FPS must be positive.");

        if (repeat)
        {
            ILDARepeater rep_in(ilda_in);
            return ILDAFltkOutput(rep_in).run();
        }
        else
        {
            return ILDAFltkOutput(ilda_in).run(1.0 / fps, width, height);
        }

        return 0;
    }
    catch (exception const &e)
    {
        cout << "Error:\n";
        cout << e.what();
        cout << endl;
        return 1;
    }
}

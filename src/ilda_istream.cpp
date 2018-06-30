#include "ilda_istream.hpp"
#include <boost/endian/conversion.hpp>

vector<ILDAColor> default_palette({{255, 0, 0},
                                   {255, 16, 0},
                                   {255, 32, 0},
                                   {255, 48, 0},
                                   {255, 64, 0},
                                   {255, 80, 0},
                                   {255, 96, 0},
                                   {255, 112, 0},
                                   {255, 128, 0},
                                   {255, 144, 0},
                                   {255, 160, 0},
                                   {255, 176, 0},
                                   {255, 192, 0},
                                   {255, 208, 0},
                                   {255, 224, 0},
                                   {255, 240, 0},
                                   {255, 255, 0},
                                   {224, 255, 0},
                                   {192, 255, 0},
                                   {160, 255, 0},
                                   {128, 255, 0},
                                   {96, 255, 0},
                                   {64, 255, 0},
                                   {32, 255, 0},
                                   {0, 255, 0},
                                   {0, 255, 36},
                                   {0, 255, 73},
                                   {0, 255, 109},
                                   {0, 255, 146},
                                   {0, 255, 182},
                                   {0, 255, 219},
                                   {0, 255, 255},
                                   {0, 227, 255},
                                   {0, 198, 255},
                                   {0, 170, 255},
                                   {0, 142, 255},
                                   {0, 113, 255},
                                   {0, 85, 255},
                                   {0, 56, 255},
                                   {0, 28, 255},
                                   {0, 0, 255},
                                   {32, 0, 255},
                                   {64, 0, 255},
                                   {96, 0, 255},
                                   {128, 0, 255},
                                   {160, 0, 255},
                                   {192, 0, 255},
                                   {224, 0, 255},
                                   {255, 0, 255},
                                   {255, 32, 255},
                                   {255, 64, 255},
                                   {255, 96, 255},
                                   {255, 128, 255},
                                   {255, 160, 255},
                                   {255, 192, 255},
                                   {255, 224, 255},
                                   {255, 255, 255},
                                   {255, 224, 224},
                                   {255, 192, 192},
                                   {255, 160, 160},
                                   {255, 128, 128},
                                   {255, 96, 96},
                                   {255, 64, 64},
                                   {255, 32, 32}});

ILDAIStream::ILDAIStream(istream &input) : input(input){};

Frame *ILDAIStream::nextFrame()
{
    readHeader();
    if (number_of_records == 0)
        return nullptr;

    switch (header.format)
    {
    case FORMAT_3D_COORDINATES_INDEXED:
        frameFrom3dCoordinatesIndexed();
        break;
    case FORMAT_2D_INDEXED:
        frameFrom3dCoordinatesIndexed();
        break;
    case FORMAT_COLOR_PALETTE:
        setColorPalette();
        break;
    case FORMAT_COORDINATES_3D_TRUE:
        frameFrom3dCoordinatesTrue();
        break;
    case FORMAT_COORDINATES_2D_TRUE:
        frameFrom2dCoordinatesTrue();
        break;
    default:
        throw runtime_error("Not supported.");
    }

    return &current_frame;
}

void ILDAIStream::frameFrom3dCoordinatesIndexed()
{
    ILDA3dCoordinatesIndexed coordinates;

    current_frame.projector = header.projector_id;
    current_frame.points.clear();

    vector<ILDAColor> *palette = (palettes.find(header.projector_id) == palettes.end())
                                     ? &default_palette
                                     : &palettes[header.projector_id];

    for (int i = 0; i < number_of_records; i++)
    {
        read(coordinates);
        Point p;
        p.x = coordinates.x;
        p.y = coordinates.y;
        p.z = coordinates.z;
        if (coordinates.color >= palette->size() || coordinates.status.blanked)
        {
            p.r = 0;
            p.g = 0;
            p.b = 0;
        }
        else
        {
            p.r = (*palette)[coordinates.color].r;
            p.g = (*palette)[coordinates.color].g;
            p.b = (*palette)[coordinates.color].b;
        }
        current_frame.points.push_back(p);
    }
}

void ILDAIStream::frameFrom2dCoordinatesIndexed()
{
    ILDA2dCoordinatesIndexed coordinates;

    current_frame.projector = header.projector_id;
    current_frame.points.clear();

    vector<ILDAColor> *palette = (palettes.find(header.projector_id) == palettes.end())
                                     ? &default_palette
                                     : &palettes[header.projector_id];

    for (int i = 0; i < number_of_records; i++)
    {
        read(coordinates);
        Point p;
        p.x = coordinates.x;
        p.y = coordinates.y;
        p.z = 0;
        if (coordinates.color >= palette->size() || coordinates.status.blanked)
        {
            p.r = 0;
            p.g = 0;
            p.b = 0;
        }
        else
        {
            p.r = (*palette)[coordinates.color].r;
            p.g = (*palette)[coordinates.color].g;
            p.b = (*palette)[coordinates.color].b;
        }
        current_frame.points.push_back(p);
    }
}

void ILDAIStream::setColorPalette()
{
    ILDAColor color;

    palettes[header.projector_id].clear();
    palettes[header.projector_id].resize(number_of_records);

    for (int i = 0; i < number_of_records; i++)
    {
        read(color);
        palettes[header.projector_id][i].r = color.r;
        palettes[header.projector_id][i].g = color.g;
        palettes[header.projector_id][i].b = color.b;
    }
}

void ILDAIStream::frameFrom3dCoordinatesTrue()
{
    ILDA3dCoordinatesTrue coordinates;

    current_frame.projector = header.projector_id;
    current_frame.points.clear();

    for (int i = 0; i < number_of_records; i++)
    {
        read(coordinates);
        Point p;
        p.x = coordinates.x;
        p.y = coordinates.y;
        p.z = coordinates.z;

        if (coordinates.status.blanked)
        {
            p.r = 0;
            p.g = 0;
            p.b = 0;
        }
        else
        {
            p.r = coordinates.r;
            p.g = coordinates.g;
            p.b = coordinates.b;
        }

        current_frame.points.push_back(p);
    }
}

void ILDAIStream::frameFrom2dCoordinatesTrue()
{
    ILDA2dCoordinatesTrue coordinates;

    current_frame.projector = header.projector_id;
    current_frame.points.clear();

    for (int i = 0; i < number_of_records; i++)
    {
        read(coordinates);
        Point p;
        p.x = coordinates.x;
        p.y = coordinates.y;
        p.z = 0;

        if (coordinates.status.blanked)
        {
            p.r = 0;
            p.g = 0;
            p.b = 0;
        }
        else
        {
            p.r = coordinates.r;
            p.g = coordinates.g;
            p.b = coordinates.b;
        }

        current_frame.points.push_back(p);
    }
}

void ILDAIStream::readHeader()
{
    read(header);

    if (string(header.ilda) != string("ILDA"))
    {
        throw runtime_error("Corrupt ILDA file.");
    }

    number_of_records = boost::endian::big_to_native(header.number_of_records);
}

template <typename T>
void ILDAIStream::read(T &target)
{
    input.read((char *)&target, sizeof(target));
    if (input.gcount() < sizeof(target))
    {
        throw runtime_error("Unexpected end of input.");
    }
};
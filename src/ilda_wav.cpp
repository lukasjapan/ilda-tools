#include "ilda_wav.hpp"
#include <boost/endian/conversion.hpp>
#include <limits>

ILDAWavOutput::ILDAWavOutput(
    ILDAInput &input,
    std::ostream &output,
    int fps, std::string signals,
    std::string invert,
    int rate,
    int pps)
    : input(input), output(output), fps(fps), signals(signals), invert(invert), rate(rate), pps(pps)
{
}

typedef struct WavHeader
{
    /* RIFF Chunk Descriptor */
    uint8_t riff[4] = {'R', 'I', 'F', 'F'}; // RIFF Header Magic header
    uint32_t chunk_size;                    // RIFF Chunk Size
    uint8_t wave[4] = {'W', 'A', 'V', 'E'}; // WAVE Header
    /* "fmt" sub-chunk */
    uint8_t fmt[4] = {'f', 'm', 't', ' '}; // FMT header
    uint32_t fmt_chunk_size = 16;          // Size of the fmt chunk
    uint16_t audio_format = 1;             // Audio format 1=PCM
    uint16_t channels;                     // Number of channels 1=Mono 2=Sterio
    uint32_t rate;                         // Sampling Frequency in Hz
    uint32_t bytes_per_second;             // bytes per second
    uint16_t bytes_per_block;              // bytes per sample (all channels)
    uint16_t bits_per_sample = 16;         // Number of bits per sample
    /* "data" sub-chunk */
    uint8_t data[4] = {'d', 'a', 't', 'a'}; // "data"  string
    uint32_t data_size;                     // Sampled data length

    void update(int data_size = 0)
    {
        this->data_size = data_size;
        this->chunk_size = data_size + 36;
        this->bytes_per_block = bits_per_sample * channels / 8;
        this->bytes_per_second = rate * bytes_per_block;
    }

    WavHeader littleEndian()
    {
        WavHeader result(*this);
        this->chunk_size = boost::endian::native_to_little(chunk_size);
        this->fmt_chunk_size = boost::endian::native_to_little(fmt_chunk_size);
        this->audio_format = boost::endian::native_to_little(audio_format);
        this->channels = boost::endian::native_to_little(channels);
        this->rate = boost::endian::native_to_little(rate);
        this->bytes_per_second = boost::endian::native_to_little(bytes_per_second);
        this->bytes_per_block = boost::endian::native_to_little(bytes_per_block);
        this->bits_per_sample = boost::endian::native_to_little(bits_per_sample);
        this->data_size = boost::endian::native_to_little(data_size);
        return result;
    }
};

int groupSizeOfNthGroup(int total_size, int groups, int index)
{
    int base = total_size / groups;
    int rest = total_size % groups;
    bool extra_needed = index * rest % groups > (index + 1) * rest % groups;
    return extra_needed ? base + 1 : base;
}

int ILDAWavOutput::run()
{
    int channel_count = signals.length();

    WavHeader header;
    header.channels = channel_count;
    header.rate = rate;
    header.update();

    WavHeader le_header = header.littleEndian();
    output.write((char *)&le_header, sizeof(le_header));

    Frame *frame;

    int frame_number = 0;
    int point_number = 0;
    int total_bytes = 0;

    bool invert_x = invert.find('x') != std::string::npos;
    bool invert_y = invert.find('y') != std::string::npos;
    bool invert_z = invert.find('z') != std::string::npos;

    int16_t last_x = 0;
    int16_t last_y = 0;
    int16_t last_z = 0;

    std::vector<int16_t> frame_data(rate * channel_count);

    int samples_in_second = 0;

    while ((frame = input.nextFrame()))
    {
        int sample_number = 0;
        int frame_in_second = frame_number % fps;

        if (frame_in_second == 0)
        {
            point_number = 0;
        }

        int point_count = groupSizeOfNthGroup(pps, fps, frame_in_second);

        // cout << "Frame " << frame_in_second << " can use " << point_count << " points" << endl;

        int location_count = frame->points.size();

        // cout << "Locations in frame: " << location_count << endl;

        for (int i = 0; i < location_count; i++)
        {
            // number of points that can be used for this location
            int point_count_of_location = groupSizeOfNthGroup(point_count, location_count, i);

            // cout << " Location " << i << " can use " << point_count_of_location << " points." << endl;

            if (point_count_of_location == 0)
            {
                continue;
            }

            const Point &coordinate = frame->points[i];

            int16_t x = invert_x ? coordinate.x * -1 : coordinate.x;
            int16_t y = invert_y ? coordinate.y * -1 : coordinate.y;
            int16_t z = invert_z ? coordinate.z * -1 : coordinate.z;

            int16_t dx = x - last_x;
            int16_t dy = y - last_y;
            int16_t dz = z - last_z;

            for (int p = 1; p <= point_count_of_location; p++)
            {
                int16_t ix = last_x + ((int)dx * p / point_count_of_location);
                int16_t iy = last_y + ((int)dy * p / point_count_of_location);
                int16_t iz = last_z + ((int)dz * p / point_count_of_location);
                int16_t l = (coordinate.r == 0 && coordinate.g == 0 && coordinate.b == 0) ? 0 : std::numeric_limits<int16_t>::max();
                int16_t r = coordinate.r * (std::numeric_limits<int16_t>::max() / std::numeric_limits<uint8_t>::max());
                int16_t g = coordinate.g * (std::numeric_limits<int16_t>::max() / std::numeric_limits<uint8_t>::max());
                int16_t b = coordinate.b * (std::numeric_limits<int16_t>::max() / std::numeric_limits<uint8_t>::max());

                int samples_of_location = groupSizeOfNthGroup(rate, pps, point_number);

                // cout << "  Point " << point_number << " can use " << samples_of_location << " samples" << endl;

                for (int q = 0; q < samples_of_location; q++)
                {
                    for (unsigned char s : signals)
                    {
                        switch (s)
                        {
                        case 'x':
                            frame_data[sample_number] = ix;
                            break;
                        case 'y':
                            frame_data[sample_number] = iy;
                            break;
                        case 'z':
                            frame_data[sample_number] = iz;
                            break;
                        case 'l':
                            frame_data[sample_number] = l;
                            break;
                        case 'r':
                            frame_data[sample_number] = r;
                            break;
                        case 'g':
                            frame_data[sample_number] = g;
                            break;
                        case 'b':
                            frame_data[sample_number] = b;
                            break;
                        default:
                            throw runtime_error("Invalid signal.");
                            break;
                        }

                        sample_number++;
                    }
                }

                point_number++;
            }

            last_x = x;
            last_y = y;
            last_z = z;
        }

        int bytes = sample_number * sizeof(int16_t);
        output.write((char *)&frame_data[0], bytes);

        total_bytes += bytes;
        frame_number++;
    }

    header.update(total_bytes);

    le_header = header.littleEndian();
    output.seekp(0);
    output.write((char *)&le_header, sizeof(le_header));

    return 0;
}

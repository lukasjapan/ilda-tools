#ifndef __SRC_FRAME__
#define __SRC_FRAME__

#include <stdint.h>
#include <vector>

struct Point
{
    int16_t x, y, z;
    uint16_t r, g, b;
};

struct Frame
{
    uint8_t projector;
    std::vector<Point> points;
};

class ILDAInput
{
  public:
    virtual Frame *nextFrame() = 0;
};

#endif
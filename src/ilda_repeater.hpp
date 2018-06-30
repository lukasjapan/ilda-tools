#ifndef __SRC_ILDA_REPEATER__
#define __SRC_ILDA_REPEATER__

#include "ilda.hpp"
#include "frame.hpp"

class ILDARepeater : public ILDAInput
{
    ILDAInput &original_input;
    vector<Frame> frames;
    vector<Frame>::iterator it;
    int repeating;

  public:
    ILDARepeater(ILDAInput &input);
    Frame *nextFrame();
};

#endif
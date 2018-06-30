#include "ilda_repeater.hpp"

ILDARepeater::ILDARepeater(ILDAInput &input) : original_input(input), repeating(false){};

Frame *ILDARepeater::nextFrame()
{
    if (!repeating)
    {
        Frame *frame = original_input.nextFrame();

        if (frame)
        {
            frames.push_back(*frame);
            return frame;
        }

        repeating = true;
        it = frames.begin();
    }

    if (it == frames.end())
    {
        it = frames.begin();
    }

    return &*it++;
}
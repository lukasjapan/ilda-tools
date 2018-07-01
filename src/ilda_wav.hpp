#ifndef __SRC_ILDA_WAV__
#define __SRC_ILDA_WAV__

#include <string>
#include <iostream>
#include "ilda.hpp"
#include "interface.hpp"

class ILDAWavOutput
{
  ILDAInput &input;
  std::ostream &output;
  int fps;
  std::string signals;
  std::string invert;
  int rate;
  int pps;

public:
  ILDAWavOutput(ILDAInput &input, std::ostream &output, int fps = 20, std::string signals = "xyl", std::string invert = "", int rate = 44100, int pps = -1);
  int run();
};

#endif
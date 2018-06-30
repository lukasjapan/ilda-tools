#ifndef __SRC_ILDA_ISTREAM__
#define __SRC_ILDA_ISTREAM__

#include <iostream>
#include <map>
#include "frame.hpp"
#include "ilda.hpp"

class ILDAIStream : public ILDAInput
{
  istream &input;
  ilda_header header;
  Frame current_frame;
  uint16_t number_of_records;
  map<uint16_t, vector<ILDAColor>> palettes;

public:
  ILDAIStream(istream &input);
  Frame *nextFrame();

private:
  void frameFrom3dCoordinatesIndexed();
  void frameFrom2dCoordinatesIndexed();
  void setColorPalette();
  void frameFrom3dCoordinatesTrue();
  void frameFrom2dCoordinatesTrue();
  void readHeader();
  template <typename T>
  void read(T &target);
};

#endif
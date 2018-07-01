#ifndef __SRC_FLTK_OUTPUT__
#define __SRC_FLTK_OUTPUT__

#include <FL/Fl.H>
#include <FL/fl_draw.H>
#include <FL/Fl_Double_Window.H>
#include "interface.hpp"

class ILDAWidget : public Fl_Widget
{
  Frame frame;

public:
  ILDAWidget(int X, int Y, int W, int H, const char *L = 0);
  void update(Frame &frame);
  void draw();
};

class ILDAFltkOutput
{
  ILDAInput &input;
  ILDAWidget *canvas;
  Fl_Double_Window *win;
  float speed;
  void callback();
  static void _callback(void *self);

public:
  ILDAFltkOutput(ILDAInput &input);
  int run(float speed = 0.05, int width = 500, int height = 500);
};

#endif
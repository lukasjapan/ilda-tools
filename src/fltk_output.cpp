#include <boost/endian/conversion.hpp>
#include "fltk_output.hpp"

ILDAWidget::ILDAWidget(int X, int Y, int W, int H, const char *L) : Fl_Widget(X, Y, W, H, L), frame(Frame()) {}

void ILDAWidget::update(Frame &frame)
{
    this->frame = frame;
    redraw();
}

void ILDAWidget::draw()
{
    fl_color(FL_BLACK);

    int r_x = w() - 1;
    int r_y = h() - 1;
    int l_x = -1;
    int l_y = -1;
    // int l_c = FL_BLACK;

    fl_draw_box(FL_FLAT_BOX, x(), y(), r_x, r_y, FL_BLACK);

    for (auto point : frame.points)
    {
        int ilda_x = point.x;
        int ilda_y = point.y;

        int _x = x() + ((ilda_x + 32768) * r_x) / 65535;
        int _y = h() - y() - ((ilda_y + 32768) * r_y) / 65535;
        int _c = fl_rgb_color(point.r, point.g, point.b);

        if (_c != FL_BLACK)
        {
            if (l_x >= 0 && l_y >= 0)
            {
                fl_line(l_x, l_y, _x, _y);
            }

            fl_color(_c);
            // fl_circle(_x, _y, 1.5);
        }

        l_x = _x;
        l_y = _y;
        // l_c = _c;
    }
}

void ILDAFltkOutput::callback()
{
    Frame *frame = input.nextFrame();
    if (frame)
    {
        canvas->update(*frame);
        Fl::repeat_timeout(speed, _callback, this);
    }
    else
    {
        win->hide();
    }
}

void ILDAFltkOutput::_callback(void *self)
{
    ((ILDAFltkOutput *)self)->callback();
}

ILDAFltkOutput::ILDAFltkOutput(ILDAInput &input) : input(input)
{
}

int ILDAFltkOutput::run(float speed, int width, int height)
{
    Fl_Double_Window win(width, height, "ILDA Player");
    this->win = &win;
    this->speed = speed;
    ILDAWidget draw_x(0, 0, win.w(), win.h());
    this->canvas = &draw_x;
    win.resizable(draw_x);
    win.show();
    Fl::add_timeout(speed, _callback, this);
    return Fl::run();
}
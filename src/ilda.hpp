#ifndef __SRC_ILDA__
#define __SRC_ILDA__

#include <stdint.h>
#include <vector>

using namespace std;

#ifdef _MSC_VER
#define PACKED
#pragma pack(push, 1)
#else
#define PACKED __attribute__((__packed__))
#endif

#define FORMAT_3D_COORDINATES_INDEXED 0
#define FORMAT_2D_INDEXED 1
#define FORMAT_COLOR_PALETTE 2
#define FORMAT_COORDINATES_3D_TRUE 4
#define FORMAT_COORDINATES_2D_TRUE 5

typedef struct PACKED
{
    char ilda[4];
    char reserved_a[3];
    uint8_t format;
    char name[8];
    char company[8];
    uint16_t number_of_records;
    uint16_t frame_number;
    uint16_t total_frames;
    uint8_t projector_id;
    uint8_t reserved_b;
} ilda_header;

typedef struct PACKED
{
    uint8_t : 6;
    uint8_t blanked : 1;
    uint8_t last_point : 1;
} ilda_status;

typedef struct PACKED
{
    uint8_t r;
    uint8_t g;
    uint8_t b;
} ILDAColor;

typedef struct PACKED
{
    int16_t x;
    int16_t y;
    ilda_status status;
    uint8_t color;
} ILDA2dCoordinatesIndexed;

typedef struct PACKED
{
    int16_t x;
    int16_t y;
    ilda_status status;
    uint8_t b;
    uint8_t g;
    uint8_t r;
} ILDA2dCoordinatesTrue;

typedef struct PACKED
{
    int16_t x;
    int16_t y;
    int16_t z;
    ilda_status status;
    uint8_t color;
} ILDA3dCoordinatesIndexed;

typedef struct PACKED
{
    int16_t x;
    int16_t y;
    int16_t z;
    ilda_status status;
    uint8_t b;
    uint8_t g;
    uint8_t r;
} ILDA3dCoordinatesTrue;

#ifdef _MSC_VER
#pragma pack(pop)
#endif

#undef PACKED

#endif
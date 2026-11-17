#include "themes.hpp"

namespace gtavi::themes {

namespace {

constexpr ThemeInfo dusk{
    "Dusk",
    {rgb("#120a30"), rgb("#7b1d72"), rgb("#f2643d"), rgb("#ffe27a"), rgb("#ff3d81"), rgb("#5c2410"), rgb("#1b0b3d"),
     0.55f, 1.0f},
    {{{0.0f, rgb("#120a30")},
      {0.34f, rgb("#4a1560")},
      {0.52f, rgb("#a02a6c")},
      {0.0f, rgb("#f2643d"), true},
      {0.002f, rgb("#2a1050"), true},
      {1.0f, rgb("#140a2c")}}},
    {rgb("#120a30"), rgb("#a02a6c"), rgb("#f2643d")},
};

constexpr ThemeInfo midnight{
    "Midnight",
    {rgb("#020611"), rgb("#0b2356"), rgb("#157f8f"), rgb("#f6f9ff"), rgb("#a9c8ff"), rgb("#0b2c44"), rgb("#030b1e"),
     1.0f, 0.0f},
    {{{0.0f, rgb("#020611")},
      {0.36f, rgb("#081a40")},
      {0.54f, rgb("#0e4a70")},
      {0.0f, rgb("#157f8f"), true},
      {0.002f, rgb("#06142e"), true},
      {1.0f, rgb("#020611")}}},
    {rgb("#020611"), rgb("#0b2356"), rgb("#157f8f")},
};

constexpr ThemeInfo dawn{
    "Dawn",
    {rgb("#10224f"), rgb("#a8527f"), rgb("#f7a765"), rgb("#fff1c2"), rgb("#ff9a5e"), rgb("#4a2a16"), rgb("#142a55"),
     0.2f, 0.6f},
    {{{0.0f, rgb("#10224f")},
      {0.34f, rgb("#5a3a70")},
      {0.52f, rgb("#b85f7d")},
      {0.0f, rgb("#f7a765"), true},
      {0.002f, rgb("#1c356a"), true},
      {1.0f, rgb("#0f1f45")}}},
    {rgb("#10224f"), rgb("#a8527f"), rgb("#f7a765")},
};

}  // namespace

const ThemeInfo& info(Theme theme) {
    switch (theme) {
        case Theme::midnight: return midnight;
        case Theme::dawn: return dawn;
        case Theme::dusk: break;
    }
    return dusk;
}

}  // namespace gtavi::themes

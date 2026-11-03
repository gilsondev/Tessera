#pragma once

#include <stdint.h>

// Fullscreen flip clock (centered) while screen_rest is in Saver/Fading.
// Follows face_prefs time format (HH:MM or HH:MM:SS).
namespace screensaver {

struct Timing {
    uint32_t max_gap_ms = 0;
    uint32_t changes = 0;
    uint32_t snaps = 0;
    uint32_t active = 0;
    char digits[7] = {};
};
Timing timing();

void begin();
void show();
void hide();
bool visible();
void tick(uint32_t now_ms);
void apply_theme();
// Avança 1 minuto com animação (console REST flip / smoke).
void debug_flip();

}  // namespace screensaver

#pragma once
#include <lvgl.h>
#include "media/snapshot.h"
namespace media_home {
lv_obj_t *create(lv_obj_t *parent, lv_event_cb_t on_open, void *data);
void reset();
void apply_theme();
void paint(const media::View &view, bool agent_ok);
}

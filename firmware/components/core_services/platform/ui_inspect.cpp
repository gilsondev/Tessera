#include "platform/ui_inspect.h"
#include <Arduino.h>
#include <ArduinoJson.h>
#include <lvgl.h>
namespace {
void walk(Print &out, lv_obj_t *obj) {
    if (!lv_obj_is_visible(obj)) return;
    if (lv_obj_check_type(obj, &lv_label_class)) {
        const char *text = lv_label_get_text(obj);
        if (text && *text) {
            auto *target = obj;
            while (lv_obj_get_parent(target) && !lv_obj_has_flag(target, LV_OBJ_FLAG_CLICKABLE)) target = lv_obj_get_parent(target);
            lv_area_t box; lv_obj_get_coords(target, &box);
            JsonDocument row; row["text"] = text; row["x"] = (box.x1+box.x2)/2; row["y"] = (box.y1+box.y2)/2;
            row["clickable"] = lv_obj_has_flag(target, LV_OBJ_FLAG_CLICKABLE);
            out.print("UI_NODE\t"); serializeJson(row, out); out.println();
        }
    }
    for (uint32_t i=0;i<lv_obj_get_child_count(obj);++i) walk(out, lv_obj_get_child(obj,i));
}
}
// Detail/settings pages are full-screen overlays on the top layer, not the
// active screen. Inspect the frontmost opaque page so hidden home content cannot
// satisfy assertions for a page that the user is no longer seeing.
lv_obj_t *front_page(lv_obj_t *layer) {
    const int count = static_cast<int>(lv_obj_get_child_count(layer));
    for (int i=count-1;i>=0;--i) {
        auto *child = lv_obj_get_child(layer,i);
        if (lv_obj_is_visible(child) && lv_obj_get_width(child)>=lv_display_get_horizontal_resolution(nullptr) &&
            lv_obj_get_height(child)>=lv_display_get_vertical_resolution(nullptr) &&
            lv_obj_get_style_bg_opa(child,0)==LV_OPA_COVER) return child;
    }
    return nullptr;
}
void ui_inspect::dump(Print &out) {
    out.printf("UI_TREE_BEGIN %d %d\n", lv_display_get_horizontal_resolution(nullptr), lv_display_get_vertical_resolution(nullptr));
    auto *root = front_page(lv_layer_sys());
    if (!root) root = front_page(lv_layer_top());
    walk(out, root ? root : lv_screen_active()); out.println("UI_TREE_END");
}

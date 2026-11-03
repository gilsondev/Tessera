#include "ui/mac_page.h"

#include "ui/kit.h"
#include "ui/palette.h"
#include "ui/theme.h"

#include <cstdio>
#include <cstring>

namespace {

lv_obj_t *screen = nullptr;
lv_obj_t *title_label = nullptr;
lv_obj_t *status_label = nullptr;
lv_obj_t *body = nullptr;
lv_obj_t *grid = nullptr;
lv_event_cb_t back_cb = nullptr;
mac_page::CmdFn cmd_cb = nullptr;
bool standalone_screen = false;
focus::View cached = {};
bool cached_agent_ok = false;
bool had_good_agent = false;

void set_if(lv_obj_t *label, const char *text) {
    if (label && text && strcmp(lv_label_get_text(label), text) != 0) lv_label_set_text(label, text);
}

void set_status(const char *text) {
    if (!status_label) return;
    if (!text || !text[0]) {
        lv_label_set_text(status_label, "");
        lv_obj_add_flag(status_label, LV_OBJ_FLAG_HIDDEN);
        return;
    }
    set_if(status_label, text);
    lv_obj_remove_flag(status_label, LV_OBJ_FLAG_HIDDEN);
}

void fire_cmd(const char *cmd) {
    if (cmd_cb && cmd) cmd_cb(cmd);
}

void build_into(lv_obj_t *parent) {
    lv_obj_set_style_bg_color(parent, theme::background(), 0);
    lv_obj_set_style_bg_opa(parent, LV_OPA_COVER, 0);
    lv_obj_set_flex_flow(parent, LV_FLEX_FLOW_COLUMN);
    lv_obj_set_style_pad_all(parent, 0, 0);
    lv_obj_set_style_pad_row(parent, 0, 0);
    lv_obj_remove_flag(parent, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_set_size(parent, lv_pct(100), lv_pct(100));

    kit::SubsysBar bar = kit::subsys_bar(parent, LV_SYMBOL_LEFT, back_cb, palette::widget_mac());
    title_label = bar.title;
    status_label = bar.status;
    lv_label_set_text(title_label, "Foco");

    body = kit::bare(parent);
    lv_obj_set_width(body, lv_pct(100));
    lv_obj_set_flex_grow(body, 1);
    lv_obj_set_flex_flow(body, LV_FLEX_FLOW_COLUMN);
    lv_obj_set_style_pad_hor(body, 12, 0);
    lv_obj_set_style_pad_top(body, 12, 0);
    lv_obj_set_style_pad_bottom(body, 12, 0);
    lv_obj_set_style_pad_row(body, 8, 0);

    grid = kit::bare(body);
    lv_obj_set_width(grid, lv_pct(100));
    lv_obj_set_flex_grow(grid, 1);
    lv_obj_set_flex_flow(grid, LV_FLEX_FLOW_ROW_WRAP);
    lv_obj_set_flex_align(grid, LV_FLEX_ALIGN_START, LV_FLEX_ALIGN_START, LV_FLEX_ALIGN_START);
    lv_obj_set_style_pad_column(grid, 8, 0);
    lv_obj_set_style_pad_row(grid, 8, 0);
    kit::enable_list_scroll(grid);
}

void refill_grid(const focus::View &view, bool agent_ok) {
    if (!grid) return;
    lv_obj_clean(grid);

    if (!agent_ok && !had_good_agent) {
        lv_obj_t *msg = lv_label_create(grid);
        lv_label_set_text(msg, "Aguardando Agente…");
        lv_obj_set_style_text_font(msg, &theme::title_font(), 0);
        lv_obj_set_style_text_color(msg, theme::text_primary(), 0);
        set_status("");
        return;
    }

    if (!view.ready || !view.ok) {
        lv_obj_t *msg = lv_label_create(grid);
        lv_label_set_text(msg, view.ready ? "Focus indisponível" : "Aguardando Focus…");
        lv_obj_set_style_text_font(msg, &theme::label_font(), 0);
        lv_obj_set_style_text_color(msg, theme::text_secondary(), 0);
        set_status(view.stale ? "desatualizado" : "");
        return;
    }

    lv_obj_t *name = lv_label_create(grid);
    lv_obj_set_width(name, lv_pct(100));
    lv_label_set_text(name, view.active_name[0] ? view.active_name : "Nenhum foco ativo");
    lv_obj_set_style_text_font(name, &theme::title_font(), 0);
    lv_obj_set_style_text_color(name, theme::text_primary(), 0);
    lv_obj_t *hint = lv_label_create(grid);
    lv_obj_set_width(hint, lv_pct(100));
    lv_label_set_text(hint, "Altere o foco na Central de Controle do Mac. O painel acompanha automaticamente.");
    lv_label_set_long_mode(hint, LV_LABEL_LONG_WRAP);
    lv_obj_set_style_text_color(hint, theme::text_secondary(), 0);
    set_status(view.stale ? "desatualizado" : "No Mac");
}

}  // namespace

void mac_page::begin(lv_event_cb_t on_back, CmdFn on_cmd) {
    back_cb = on_back;
    cmd_cb = on_cmd;
}

void mac_page::unmount() {
    if (!screen) return;
    if (standalone_screen) lv_obj_delete(screen);
    else lv_obj_clean(screen);
    screen = nullptr;
    title_label = status_label = body = grid = nullptr;
    standalone_screen = false;
}

void mac_page::mount(lv_obj_t *parent) {
    if (!parent) return;
    unmount();
    screen = parent;
    standalone_screen = false;
    build_into(parent);
    paint(cached, cached_agent_ok || had_good_agent);
    fire_cmd("?");
}

void mac_page::show() {
    unmount();
    standalone_screen = true;
    screen = lv_obj_create(lv_screen_active());
    lv_obj_set_size(screen, lv_pct(100), lv_pct(100));
    build_into(screen);
    paint(cached, cached_agent_ok || had_good_agent);
    fire_cmd("?");
}

void mac_page::apply_theme() {
    if (!screen) return;
    lv_obj_set_style_bg_color(screen, theme::background(), 0);
    if (title_label) lv_obj_set_style_text_color(title_label, theme::text_primary(), 0);
    if (status_label) lv_obj_set_style_text_color(status_label, theme::text_secondary(), 0);
    paint(cached, cached_agent_ok);
}

void mac_page::paint(const focus::View &view, bool agent_ok) {
    if (agent_ok) had_good_agent = true;
    cached = view;
    cached_agent_ok = agent_ok;
    refill_grid(cached, agent_ok);
}

const focus::View &mac_page::last_view() { return cached; }

#include "ui/media_tile.h"
#include "ui/kit.h"
#include "ui/theme.h"
#include <cstdio>
#include <cstring>
namespace {
lv_obj_t *card, *art, *placeholder, *track, *artist, *album;
lv_image_dsc_t image = {};
char shown[media::kCoverKeyMax] = {};
void text(lv_obj_t *label, const char *s) { if (label && strcmp(lv_label_get_text(label), s)) lv_label_set_text(label, s); }
lv_obj_t *label(lv_obj_t *parent, const char *value) {
    auto *l = lv_label_create(parent); lv_label_set_text(l, value);
    lv_obj_set_width(l, lv_pct(100)); lv_label_set_long_mode(l, LV_LABEL_LONG_DOT);
    lv_obj_set_style_text_font(l, &theme::small_font(), 0);
    lv_obj_set_style_text_color(l, theme::text_secondary(), 0); return l;
}
}
lv_obj_t *make_media_card(lv_obj_t *parent, lv_event_cb_t on_open, void *data) {
    shown[0] = 0;
    card = kit::bare(parent); lv_obj_set_width(card, lv_pct(100)); lv_obj_set_height(card, 100);
    lv_obj_set_style_bg_color(card, theme::surface(), 0); lv_obj_set_style_bg_opa(card, LV_OPA_COVER, 0);
    lv_obj_set_style_radius(card, 16, 0); lv_obj_set_style_border_width(card, 1, 0);
    lv_obj_set_style_border_color(card, theme::border(), 0); lv_obj_set_style_pad_all(card, 12, 0);
    lv_obj_set_style_pad_column(card, 12, 0); lv_obj_set_flex_flow(card, LV_FLEX_FLOW_ROW);
    lv_obj_set_flex_align(card, LV_FLEX_ALIGN_START, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
    lv_obj_add_flag(card, LV_OBJ_FLAG_CLICKABLE); kit::press_feedback(card);
    if (on_open) lv_obj_add_event_cb(card, on_open, LV_EVENT_CLICKED, data);
    auto *cover = kit::bare(card); lv_obj_set_size(cover, 64, 64);
    lv_obj_set_style_radius(cover, 8, 0); lv_obj_set_style_clip_corner(cover, true, 0);
    lv_obj_set_style_bg_color(cover, theme::background(), 0); lv_obj_set_style_bg_opa(cover, LV_OPA_COVER, 0);
    placeholder = lv_label_create(cover); lv_label_set_text(placeholder, LV_SYMBOL_AUDIO); lv_obj_set_style_text_font(placeholder, &lv_font_montserrat_20, 0); lv_obj_set_style_text_color(placeholder, theme::text_secondary(), 0); lv_obj_center(placeholder);
    art = lv_image_create(cover); lv_obj_set_size(art, 64, 64); lv_image_set_inner_align(art, LV_IMAGE_ALIGN_STRETCH);
    lv_obj_add_flag(art, LV_OBJ_FLAG_HIDDEN);
    auto *meta = kit::bare(card); lv_obj_set_flex_grow(meta, 1); lv_obj_set_width(meta, 0);
    lv_obj_set_height(meta, LV_SIZE_CONTENT); lv_obj_set_flex_flow(meta, LV_FLEX_FLOW_COLUMN); lv_obj_set_style_pad_row(meta, 3, 0);
    track = label(meta, "Nada tocando"); lv_obj_set_style_text_color(track, theme::text_primary(), 0);
    artist = label(meta, "Música do Mac"); album = label(meta, "");
    return card;
}
void media_tile::set(const media::View &view, bool agent_ok) {
    if (!card) return;
    const bool playing = view.ready && view.media.ok && view.media.state != media::MediaState::Stopped;
    text(track, playing ? view.media.title : agent_ok ? "Nada tocando" : "Conecte o Mac por USB");
    text(artist, playing ? view.media.artist : "Música");
    char detail[110];
    if (playing && view.media.album[0] && view.media.year[0] && strcmp(view.media.year,"0")) snprintf(detail,sizeof(detail),"%s · %s",view.media.album,view.media.year);
    else snprintf(detail,sizeof(detail),"%s",playing ? view.media.album : "");
    if (!agent_ok || view.stale) snprintf(detail,sizeof(detail),"Desatualizado · reconectando");
    text(album, detail);
    if (playing && view.cover.ready && view.cover.rgb565) {
        if (strcmp(shown,view.cover.key)) {
            lv_image_cache_drop(&image);
            image = {}; image.header.magic = LV_IMAGE_HEADER_MAGIC; image.header.cf = LV_COLOR_FORMAT_RGB565;
            image.header.w = view.cover.w; image.header.h = view.cover.h; image.header.stride = view.cover.w * 2;
            image.data_size = media::kCoverBytes; image.data = view.cover.rgb565;
            lv_image_set_src(art, &image); snprintf(shown,sizeof(shown),"%s",view.cover.key);
        }
        lv_obj_remove_flag(art,LV_OBJ_FLAG_HIDDEN); lv_obj_add_flag(placeholder,LV_OBJ_FLAG_HIDDEN);
    } else { lv_obj_add_flag(art,LV_OBJ_FLAG_HIDDEN); lv_obj_remove_flag(placeholder,LV_OBJ_FLAG_HIDDEN); shown[0]=0; }
}

void media_tile::detach() { card = art = placeholder = track = artist = album = nullptr; shown[0] = 0; }

void media_tile::apply_theme() {
    if (!card) return;
    lv_obj_set_style_bg_color(card,theme::surface(),0);
    lv_obj_set_style_border_color(card,theme::border(),0);
    lv_obj_set_style_text_color(track,theme::text_primary(),0);
    lv_obj_set_style_text_color(artist,theme::text_secondary(),0);
    lv_obj_set_style_text_color(album,theme::text_secondary(),0);
    lv_obj_set_style_text_color(placeholder,theme::text_secondary(),0);
    lv_obj_set_style_bg_color(lv_obj_get_parent(art),theme::background(),0);
}

bool media_tile::claims(const char *id) { return id && !strcmp(id, "org.tessera.media"); }
void media_tile::add(lv_obj_t *row, lv_event_cb_t on_open) { make_media_card(row, on_open, const_cast<char *>("org.tessera.media")); }

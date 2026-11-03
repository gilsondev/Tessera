#include "media/snapshot.h"
#include "tessera/data.h"

#include <cstdio>
#include <cstring>
#include <unity.h>

void test_mac_apply_playing(void) {
    media::View v{};
    TEST_ASSERT_TRUE(
        media::apply_line(v, "MEDIA\tv1\t1\tplaying\tSong\tArtist\tMusic", 1000));
    TEST_ASSERT_TRUE(v.ready);
    TEST_ASSERT_TRUE(v.media.ok);
    TEST_ASSERT_EQUAL(static_cast<int>(media::MediaState::Playing), static_cast<int>(v.media.state));
    TEST_ASSERT_EQUAL_STRING("Song", v.media.title);
    TEST_ASSERT_EQUAL_STRING("Artist", v.media.artist);
    TEST_ASSERT_EQUAL_STRING("Music", v.media.app);
}

void test_mac_apply_paused_keeps_title(void) {
    media::View v{};
    media::apply_line(v, "MEDIA\tv1\t1\tpaused\tSong\tArtist\tSpotify", 1);
    TEST_ASSERT_EQUAL(static_cast<int>(media::MediaState::Paused), static_cast<int>(v.media.state));
    TEST_ASSERT_EQUAL_STRING("Song", v.media.title);
}

void test_mac_apply_stopped(void) {
    media::View v{};
    media::apply_line(v, "MEDIA\tv1\t1\tstopped\t\t\t", 1);
    TEST_ASSERT_EQUAL(static_cast<int>(media::MediaState::Stopped), static_cast<int>(v.media.state));
    TEST_ASSERT_EQUAL_STRING("", v.media.title);
}

void test_mac_apply_ok0(void) {
    media::View v{};
    media::apply_line(v, "MEDIA\tv1\t0\t\t\t\t", 1);
    TEST_ASSERT_TRUE(v.ready);
    TEST_ASSERT_FALSE(v.media.ok);
    TEST_ASSERT_EQUAL(static_cast<int>(media::MediaState::Stopped), static_cast<int>(v.media.state));
}

void test_mac_reject_wrong_prefix(void) {
    media::View v{};
    TEST_ASSERT_FALSE(media::apply_line(v, "AITOKENS\tv1\t1", 1));
    TEST_ASSERT_FALSE(v.ready);
}

void test_mac_stale_after_45s(void) {
    media::View v{};
    media::apply_line(v, "MEDIA\tv1\t1\tplaying\tA\tB\tC", 1000);
    TEST_ASSERT_FALSE(media::refresh_age(v, 1000 + 44999));
    TEST_ASSERT_FALSE(v.stale);
    TEST_ASSERT_TRUE(media::refresh_age(v, 1000 + 45000));
    TEST_ASSERT_TRUE(v.stale);
}

void test_mac_cover_assembles_chunks(void) {
    media::View v{};
    media::apply_line(v,"MEDIA\tv1\t1\tplaying\tSong\tArtist\tMusic\tAlbum\t2026\tabc123",1);
    TEST_ASSERT_EQUAL_STRING("Album",v.media.album);
    TEST_ASSERT_EQUAL_STRING("2026",v.media.year);
    char hex[961], line[1100];
    const int chunks = (media::kCoverBytes + 479) / 480;
    // A final chunk arriving first must never produce a valid image.
    snprintf(line,sizeof(line),"MEDIA_COVER\tv1\t1\tabc123\t200\t200\trgb565le\t%d\t%d\t00",chunks,chunks-1);
    TEST_ASSERT_FALSE(media::apply_cover_line(v,line));
    for (int i=0;i<chunks;++i) {
        const int size = (media::kCoverBytes - i*480) < 480 ? media::kCoverBytes-i*480 : 480;
        for (int j=0;j<size;++j) snprintf(hex+j*2,3,"%02x",j%2 ? 0xf8 : 0x00);
        snprintf(line,sizeof(line),"MEDIA_COVER\tv1\t1\tabc123\t200\t200\trgb565le\t%d\t%d\t%s",chunks,i,hex);
        TEST_ASSERT_EQUAL(i==chunks-1,media::apply_cover_line(v,line));
    }
    TEST_ASSERT_TRUE(v.cover.ready);
    TEST_ASSERT_NOT_NULL(v.cover.rgb565);
    TEST_ASSERT_EQUAL_STRING("abc123",v.cover.key);
    TEST_ASSERT_EQUAL(0xf8,v.cover.rgb565[1]);
    // Metadata moves first; a delayed cover from the previous track is rejected.
    media::apply_line(v,"MEDIA\tv1\t1\tplaying\tNew\tArtist\tMusic\tAlbum\t2026\tnewkey",2);
    TEST_ASSERT_FALSE(media::apply_cover_line(v,line));
    TEST_ASSERT_FALSE(v.cover.ready);
}
void cover_line(char *line, size_t n, const char *key, int chunks, int idx, uint8_t byte) {
    uint8_t raw[480];
    const int size = idx == 166 ? 320 : 480;
    memset(raw, byte, sizeof(raw));
    char hex[961];
    for (int i=0;i<size;++i) snprintf(hex+i*2,3,"%02x",byte);
    snprintf(line, n, "MEDIA_COVER	v1	1	%s	200	200	rgb565le	%d	%d	%08x	%s", key, chunks, idx,
             media::cover_crc32(raw, size), hex);
}

void test_cover_publishes_only_when_the_image_is_complete(void) {
    media::View view{};
    tessera::TopicBus bus;
    tessera::SnapshotWriter<media::Published> writer(bus, tessera::TopicId::Media);
    int publishes = 0;
    auto publish = [&]() {
        media::Published &slot = writer.begin();
        slot.view = view;
        slot.agent_ok = true;
        writer.commit();
        publishes++;
    };

    char line[1200];
    media::apply_line(view, "MEDIA\tv1\t1\tplaying\tSong\tArtist\tMusic", 1);
    for (int i=0;i<167;++i) { cover_line(line, sizeof(line), "old", 167, i, 0x11); TEST_ASSERT_EQUAL(i==166, media::apply_cover_line(view, line)); }
    publish();
    publishes = 0;

    const tessera::DataRef before = bus.latest(tessera::TopicId::Media);
    TEST_ASSERT_NOT_NULL(before.ptr);
    const auto *shown = static_cast<const media::Published *>(before.ptr);
    const uint8_t *old_pixels = shown->view.cover.rgb565;
    TEST_ASSERT_NOT_NULL(old_pixels);
    TEST_ASSERT_EQUAL_UINT8(0x11, old_pixels[0]);

    for (int idx = 0; idx < 3; ++idx) {
        cover_line(line, sizeof(line), "new", 167, idx, 0xee);
        TEST_ASSERT_FALSE(media::apply_cover_line(view, line));
    }
    TEST_ASSERT_EQUAL_INT(0, publishes);
    const tessera::DataRef mid = bus.latest(tessera::TopicId::Media);
    const auto *still = static_cast<const media::Published *>(mid.ptr);
    TEST_ASSERT_EQUAL_PTR(old_pixels, still->view.cover.rgb565);
    TEST_ASSERT_EQUAL_UINT8(0x11, old_pixels[0]);

    for (int i=3;i<167;++i) { cover_line(line, sizeof(line), "new", 167, i, 0xee); TEST_ASSERT_EQUAL(i==166, media::apply_cover_line(view, line)); }
    publish();
    TEST_ASSERT_EQUAL_INT(1, publishes);

    const tessera::DataRef after = bus.latest(tessera::TopicId::Media);
    const auto *fresh = static_cast<const media::Published *>(after.ptr);
    TEST_ASSERT_NOT_EQUAL(old_pixels, fresh->view.cover.rgb565);
    TEST_ASSERT_EQUAL_UINT8(0xee, fresh->view.cover.rgb565[0]);
    TEST_ASSERT_EQUAL_UINT8(0x11, old_pixels[0]);
}

void test_a_missing_cover_chunk_does_not_swap(void) {
    const uint8_t *known = reinterpret_cast<const uint8_t *>("123456789");
    TEST_ASSERT_EQUAL_UINT32(0xCBF43926u, media::cover_crc32(known, 9));

    media::View view{};
    media::apply_line(view, "MEDIA\tv1\t1\tplaying\tSong\tArtist\tMusic", 1);
    char line[1200];
    for (int idx = 0; idx < 167; ++idx) {
        if (idx == 80) continue;
        cover_line(line, sizeof(line), "gap", 167, idx, 0x22);
        TEST_ASSERT_FALSE(media::apply_cover_line(view, line, 1000));
    }
    TEST_ASSERT_FALSE(view.cover.ready);

    char ask[800];
    TEST_ASSERT_FALSE(media::cover_poll(1000 + 1999, ask, sizeof(ask)));
    TEST_ASSERT_TRUE(media::cover_poll(1000 + 2000, ask, sizeof(ask)));
    TEST_ASSERT_NOT_NULL(std::strstr(ask, "MEDIA_COVER?\tgap\t"));
    TEST_ASSERT_NOT_NULL(std::strstr(ask, "80"));

    snprintf(line, sizeof(line), "MEDIA_COVER\tv1\t1\tgap\t200\t200\trgb565le\t167\t80\t00000000\t2222");
    TEST_ASSERT_FALSE(media::apply_cover_line(view, line, 4000));
    TEST_ASSERT_FALSE(view.cover.ready);

    cover_line(line, sizeof(line), "gap", 167, 80, 0x22);
    TEST_ASSERT_TRUE(media::apply_cover_line(view, line, 4000));
    TEST_ASSERT_TRUE(view.cover.ready);
    TEST_ASSERT_EQUAL_UINT8(0x22, view.cover.rgb565[80 * 480]);
}

void test_an_old_agent_cover_without_crc_still_shows(void) {
    media::View view{};
    media::apply_line(view, "MEDIA\tv1\t1\tplaying\tSong\tArtist\tMusic", 1);
    char line[1200];
    for (int i=0;i<167;++i) {
        cover_line(line, sizeof(line), "oldagent", 167, i, 0x22);
        char *hex = strrchr(line, '\t');
        memmove(hex-8, hex+1, strlen(hex+1)+1); // legacy line omits CRC field
        TEST_ASSERT_EQUAL(i==166, media::apply_cover_line(view, line, 1000));
    }
    TEST_ASSERT_TRUE(view.cover.ready);
    TEST_ASSERT_EQUAL_STRING("oldagent", view.cover.key);
    TEST_ASSERT_EQUAL_UINT8(0x22, view.cover.rgb565[0]);
    TEST_ASSERT_EQUAL_UINT8(0x22, view.cover.rgb565[1]);
}

void test_mac_cover_clears_on_track_change(void) {
    media::View v{};
    v.cover.ready = true;
    strncpy(v.cover.key, "old", sizeof(v.cover.key) - 1);
    media::apply_line(v, "MEDIA\tv1\t1\tplaying\tSong\tArtist\tMusic", 1);
    // First apply has empty previous title → treated as track change → cover cleared.
    TEST_ASSERT_FALSE(v.cover.ready);
    v.cover.ready = true;
    strncpy(v.cover.key, "keep", sizeof(v.cover.key) - 1);
    media::apply_line(v, "MEDIA\tv1\t1\tpaused\tSong\tArtist\tMusic", 2);
    TEST_ASSERT_TRUE(v.cover.ready);
    TEST_ASSERT_EQUAL_STRING("keep", v.cover.key);
    media::apply_line(v, "MEDIA\tv1\t1\tplaying\tOther\tArtist\tMusic", 3);
    TEST_ASSERT_FALSE(v.cover.ready);
}

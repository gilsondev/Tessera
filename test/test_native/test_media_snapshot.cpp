#include "media/snapshot.h"

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

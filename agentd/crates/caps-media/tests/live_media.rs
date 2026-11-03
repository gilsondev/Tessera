//! Opt-in integration check. Reads the real player; never changes playback.
use tessera_caps_media::{cover, MediaCapability, MediaState, MEDIA_COVER_EDGE};
#[test]
#[ignore = "requires a real playing track and macOS Automation permission"]
fn current_player_metadata_and_artwork_are_consistent() {
    let mut media = MediaCapability::with_default_watcher();
    let line = media.build_line();
    assert!(
        line.starts_with("MEDIA\tv1\t1\t"),
        "Player metadata is unavailable"
    );
    let info = media.current_info().expect("Player snapshot");
    assert_ne!(
        info.state,
        MediaState::Stopped,
        "Play a track in Music or Spotify before this opt-in check"
    );
    assert!(!info.title.is_empty(), "Missing title");
    assert!(!info.artist.is_empty(), "Missing artist");
    assert!(!info.album.is_empty(), "Missing album");
    let bytes = cover::fetch_cover_bytes(&info).expect("Current track artwork must be available");
    let rgb = cover::jpeg_or_png_to_rgb565(&bytes, MEDIA_COVER_EDGE).expect("Artwork must decode");
    assert_eq!(
        rgb.len(),
        (MEDIA_COVER_EDGE * MEDIA_COVER_EDGE * 2) as usize
    );
    assert_eq!(cover::cover_key(&info).len(), 16);
}

//! Settled WWW-reached HTTP settings, web-network §4.9.
use fc4_core::www::http::Client;

#[test]
fn http_directory_and_cache_setting_are_independent_copies() {
    let mut client = Client::new(None);
    assert!(client.directory.is_empty());
    assert!(client.cache_setting.is_empty());
    for (input, expected) in [
        (b"out".as_slice(), b"out\\".as_slice()),
        (b"out/", b"out/"),
        (b"out\\", b"out\\"),
        (b"", b""),
        (b"a\0b", b"a\0b\\"),
    ] {
        let mut source = input.to_vec();
        client.set_directory(&source);
        source.fill(b'x');
        assert_eq!(client.directory, expected);
        assert!(client.cache_setting.is_empty());
    }
    let mut source = b"_cache\0/raw".to_vec();
    client.set_cache_setting(&source);
    source.fill(b'x');
    assert_eq!(client.cache_setting, b"_cache\0/raw");
    assert_eq!(client.directory, b"a\0b\\");
    assert!(client.queue.is_empty());
    assert!(client.active.is_none());
    assert!(!client.initialized);
    client.set_cache_setting(b"");
    assert!(client.cache_setting.is_empty());
}

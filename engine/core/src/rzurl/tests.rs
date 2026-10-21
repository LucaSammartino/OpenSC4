use super::*;

struct Row {
    ty: u32,
    scheme: &'static str,
    server: &'static str,
    port: u32,
    path: &'static str,
    params: &'static str,
    query: &'static str,
    fragment: &'static str,
}

fn check(mut u: RzUrl, r: Row) {
    let ctx = String::from_utf8_lossy(u.full_url()).into_owned();
    assert_eq!(u.url_type(), r.ty, "type {ctx}");
    assert_eq!(u.scheme(), r.scheme.as_bytes(), "scheme {ctx}");
    assert_eq!(u.server(), r.server.as_bytes(), "server {ctx}");
    assert_eq!(u.port(), r.port, "port {ctx}");
    assert_eq!(u.absolute_path(), r.path.as_bytes(), "path {ctx}");
    assert_eq!(u.params(), r.params.as_bytes(), "params {ctx}");
    assert_eq!(u.query(), r.query.as_bytes(), "query {ctx}");
    assert_eq!(u.fragment(), r.fragment.as_bytes(), "fragment {ctx}");
}

fn rel(s: &str) -> RzUrl {
    RzUrl::from_text(s.as_bytes())
}

#[allow(clippy::too_many_arguments)]
fn row(
    ty: u32,
    scheme: &'static str,
    server: &'static str,
    port: u32,
    path: &'static str,
    params: &'static str,
    query: &'static str,
    fragment: &'static str,
) -> Row {
    Row {
        ty,
        scheme,
        server,
        port,
        path,
        params,
        query,
        fragment,
    }
}

#[test]
fn spec_2_5_worked_examples() {
    check(
        rel("http://www.simcity.com:8080/a/b.html?x=1#top"),
        row(2, "http", "www.simcity.com", 8080, "/a/b.html", "", "?x=1", "#top"),
    );
    check(rel("HTTPS://Host/"), row(2, "https", "Host", 0, "/", "", "", ""));
    check(rel("http://host"), row(2, "http", "host", 0, "/", "", "", ""));
    check(rel("http://host?q"), row(2, "http", "host", 0, "", "", "?q", ""));
    check(
        rel("ftp://host/pub/f.txt;type=i"),
        row(5, "ftp", "host", 0, "/pub/f.txt", ";type=i", "", ""),
    );
    check(rel("file:///C:/x.htm"), row(4, "file", "", 0, "/C:/x.htm", "", "", ""));
    check(rel("images/a.png"), row(9, "", "", 0, "images/a.png", "", "", ""));
    check(rel("about:blank"), row(1, "about", "", 0, "blank", "", "", ""));
    let mut g = RzUrl::new();
    assert!(!g.allow_relative());
    assert!(g.parse_url(b"www.maxis.com/x"));
    assert_eq!(g.url_type(), 2);
    assert_eq!(g.scheme(), b"");
    assert_eq!(g.server(), b"www.maxis.com");
    assert_eq!(g.absolute_path(), b"/x");
}

#[test]
fn constructors_and_lazy_parse() {
    let u = RzUrl::new();
    assert!(!u.allow_relative() && !u.is_parsed());
    assert_eq!(u.full_url(), b"");
    let mut u = rel("http://h/p");
    assert!(u.allow_relative() && !u.is_parsed());
    assert_eq!(u.server(), b"h");
    assert!(u.is_parsed());
    // Copy keeps allowRelative.
    let c = u.clone();
    assert!(c.allow_relative());
    // Failing parse: reparsed each time, same result.
    let mut m = rel("mailto:a@b");
    assert_eq!(m.url_type(), 0);
    assert!(!m.is_parsed());
    assert_eq!(m.scheme(), b"mailto");
    assert_eq!(m.url_type(), 0);
    assert!(!m.is_parsed());
    // Default-constructed empty text: guess mode, type 0, fails.
    let mut e = RzUrl::new();
    assert_eq!(e.url_type(), 0);
}

#[test]
fn scheme_step_quirks() {
    // URL: / url: prefixes, exact case only.
    check(rel("URL:http://h/"), row(2, "http", "h", 0, "/", "", "", ""));
    check(rel("url:http://h/"), row(2, "http", "h", 0, "/", "", "", ""));
    check(rel("Url:x"), row(1, "Url", "", 0, "x", "", "", ""));
    // https and httpx are type 2; mailto:// reaches type 6.
    check(rel("httpx://h"), row(2, "httpx", "h", 0, "/", "", "", ""));
    // `mailto:` is caught by step 2 (type 0) even with `//`; only a longer scheme reaches 6.
    check(rel("mailto://x"), row(0, "mailto", "", 0, "", "", "", ""));
    check(rel("mailtox://x"), row(6, "mailtox", "", 0, "//x", "", "", ""));
    check(rel("news://grp"), row(7, "news", "", 0, "//grp", "", "", ""));
    check(rel("NNTP://g"), row(7, "nntp", "", 0, "//g", "", "", ""));
    check(rel("irc://u@s:6667/c"), row(8, "irc", "", 0, "s:6667/c", "", "", ""));
    check(rel("ftpx://h"), row(5, "ftpx", "h", 0, "/", "", "", ""));
    // Other-scheme test: characters allowed and not.
    check(rel("a+b.c-1:rest"), row(1, "a+b.c-1", "", 0, "rest", "", "", ""));
    check(rel(":x"), row(9, "", "", 0, ":x", "", "", ""));
    check(rel("a/b:c"), row(9, "", "", 0, "a/b:c", "", "", ""));
    // Opaque route fails on an empty path.
    // The opaque path keeps the `//`, so it is never empty from a `news://` text.
    let mut u = rel("news://");
    assert_eq!(u.url_type(), 7);
    assert!(u.is_parsed());
    assert_eq!(u.absolute_path(), b"//");
}

#[test]
fn guess_mode() {
    let cases: [(&str, u32, &str); 5] = [
        ("ftp.x.org/p", 5, "ftp.x.org"),
        ("news.x", 7, "news.x"),
        ("nntp.x", 7, "nntp.x"),
        ("irc.x/", 8, "irc.x"),
        ("www.a", 2, "www.a"),
    ];
    for (t, ty, server) in cases {
        let mut g = RzUrl::new();
        let ok = g.parse_url(t.as_bytes());
        assert_eq!(g.url_type, ty, "{t}");
        if ty == 7 {
            // Opaque route: path keeps the inserted `//`.
            assert!(ok);
            assert_eq!(g.path, format!("//{t}").as_bytes());
        } else {
            assert_eq!(g.server, server.as_bytes(), "{t}");
        }
    }
    let mut g = RzUrl::new();
    assert!(!g.parse_url(b"images/a.png"));
    assert_eq!(g.url_type, 0);
}

#[test]
fn user_info_and_port() {
    let mut u = rel("http://u:p@host/x");
    assert_eq!(u.url_type(), 2);
    assert_eq!(u.user(), b"//u");
    assert_eq!(u.password(), b"p");
    assert_eq!(u.server(), b"");
    assert_eq!(u.absolute_path(), b"host/x");
    // Port: digits only, rest of the run after them is path.
    check(rel("http://h:80x/y"), row(2, "http", "h", 80, "x/y", "", "", ""));
    check(rel("http://h:/y"), row(2, "http", "h", 0, "/y", "", "", ""));
    // Colon after the first slash: not a port.
    check(rel("http://h/a:1"), row(2, "http", "h", 0, "/a:1", "", "", ""));
    // No slash: http cuts at ; then ? then #.
    check(rel("http://h;p?q#f"), row(2, "http", "h", 0, "", ";p", "?q", "#f"));
    check(rel("http://h#f"), row(2, "http", "h", 0, "", "", "", "#f"));
    // irc does not cut.
    check(rel("irc://h?q"), row(8, "irc", "h?q", 0, "/", "", "", ""));
    // 15 digits kept; the whole run is erased.
    let mut u = RzUrl::from_text(format!("http://h:{}80000/", "0".repeat(14)).as_bytes());
    assert_eq!(u.url_type(), 2);
    assert_eq!(u.port(), 8);
    assert_eq!(u.absolute_path(), b"/");
}

#[test]
fn path_steps() {
    check(rel("http://h/a;p#f?q"), row(2, "http", "h", 0, "/a", ";p#f", "?q", ""));
    check(rel("http://h/a?q;p"), row(2, "http", "h", 0, "/a", "", "?q;p", ""));
    check(rel("x;p?q#f"), row(9, "", "", 0, "x", ";p", "?q", "#f"));
    check(rel(""), row(9, "", "", 0, "", "", "", ""));
    check(rel("ftp://h"), row(5, "ftp", "h", 0, "/", "", "", ""));
    check(rel("ftp://h/a?b#c"), row(5, "ftp", "h", 0, "/a?b#c", "", "", ""));
    check(rel("file://h/a?b"), row(4, "file", "", 0, "h/a?b", "", "", ""));
    check(rel("//srv:81/p"), row(9, "", "srv", 81, "/p", "", "", ""));
}

#[test]
fn rebuild() {
    let mut u = rel("http://www.simcity.com:8080/a/b.html?x=1#top");
    assert_eq!(
        u.make_url_string_from_components(),
        b"http://www.simcity.com:8080/a/b.html?x=1#top"
    );
    let mut u = rel("HTTPS://Host/");
    assert_eq!(u.make_url_string_from_components(), b"https://Host/");
    let mut u = rel("http://u:p@host/x");
    assert_eq!(u.make_url_string_from_components(), b"http:////u:p@host/x");
    let mut u = rel("mailtox://x");
    assert_eq!(u.make_url_string_from_components(), b"mailtox://x");
    let mut u = rel("images/a.png");
    assert_eq!(u.make_url_string_from_components(), b"://images/a.png");
    assert_eq!(u.full_url(), b"://images/a.png");
}

#[test]
fn normalisation() {
    let n = |s: &str| String::from_utf8(normalize_path(s.as_bytes())).unwrap();
    assert_eq!(n("/docs/en/../img/./x.png"), "/docs/img/x.png");
    assert_eq!(n("/a/b/../c"), "/a/c");
    assert_eq!(n("/../x"), "/x");
    assert_eq!(n("a/../b"), "b");
    assert_eq!(n("/a/b/.."), "/a/");
    assert_eq!(n("/a/."), "/a/");
    assert_eq!(n("."), "");
    assert_eq!(n("/../"), "/");
    // Step 1 resumes one past the last hit, so a second adjacent `./` is kept.
    assert_eq!(n("/a/././b"), "/a/./b");
    assert_eq!(n("/a/../../b"), "/b");
    assert_eq!(n("/.."), "/..");
    assert_eq!(n("ab/.."), "");
}

#[test]
fn change_url_cases() {
    let mut u = RzUrl::new();
    assert!(u.change_url(b"../img/./x.png", b"http://h/docs/en/page.html"));
    assert_eq!(u.full_url(), b"http://h/docs/img/x.png");
    assert!(u.allow_relative());
    assert_eq!(u.url_type(), 9);
    assert_eq!(u.server(), b"h");

    // Base empty: parse new.
    let mut u = RzUrl::new();
    assert!(u.change_url(b"http://a/b", b""));
    assert_eq!(u.full_url(), b"http://a/b");
    assert!(u.is_parsed());
    let mut u = RzUrl::new();
    assert!(!u.change_url(b"x/y", b""));

    // New empty: full = base, unparsed.
    let mut u = RzUrl::new();
    assert!(u.change_url(b"", b"http://a/b"));
    assert_eq!(u.full_url(), b"http://a/b");
    assert!(!u.is_parsed());

    // New has a scheme: kept verbatim.
    let mut u = RzUrl::new();
    assert!(u.change_url(b"FTP://x/y", b"http://a/b"));
    assert_eq!(u.full_url(), b"FTP://x/y");
    assert!(!u.is_parsed());

    // Absolute path, port inherited.
    let mut u = RzUrl::new();
    assert!(u.change_url(b"/z", b"http://a:81/b/c?q"));
    assert_eq!(u.full_url(), b"http://a:81/z");

    // Empty path: base path and chain.
    let mut u = RzUrl::new();
    assert!(u.change_url(b"#frag", b"http://a/b/c?q#old"));
    assert_eq!(u.full_url(), b"http://a/b/c?q#frag");
    let mut u = RzUrl::new();
    assert!(u.change_url(b";p", b"http://a/b;x?q#f"));
    assert_eq!(u.full_url(), b"http://a/b;p");

    // Base path without a slash is kept whole before the join.
    let mut u = RzUrl::new();
    assert!(u.change_url(b"x", b"rel"));
    assert_eq!(u.full_url(), b"://relx");

    // Server inherited from a relative N with its own server.
    let mut u = RzUrl::new();
    assert!(u.change_url(b"//other/p", b"http://a/b"));
    assert_eq!(u.full_url(), b"http://other/p");
}

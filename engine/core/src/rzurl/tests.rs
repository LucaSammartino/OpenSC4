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

#[test]
fn change_url_chain_on_own_fields() {
    // Spec 2.7 examples (Q4 answered), base `http://a/b;p?old#f`.
    let base = b"http://a/b;p?old#f";
    for (new, want) in [
        (&b"?new"[..], &b"http://a/b;p?new"[..]),
        (b"#g", b"http://a/b;p?old#g"),
        (b";q", b"http://a/b;q"),
        (b";q?r", b"http://a/b;q?r"),
        (b"?r#s", b"http://a/b;p?r#s"),
    ] {
        let mut u = RzUrl::new();
        assert!(u.change_url(new, base));
        assert_eq!(u.full_url(), want, "{}", String::from_utf8_lossy(new));
        assert!(u.is_parsed());
        assert!(u.allow_relative());
        assert_eq!(u.url_type(), RELATIVE);
    }
}

#[test]
fn constructors_do_not_parse() {
    let u = rel("http://h:81/p;x?q#f");
    assert!(!u.is_parsed());
    assert_eq!(u.stored_url_type(), 0);
    assert_eq!(u.port(), 0);
    assert!(u.stored_components().iter().all(|c| c.is_empty()));
    let d = RzUrl::new();
    assert!(!d.allow_relative() && !d.is_parsed() && d.full_url().is_empty());
}

#[test]
fn lazy_component_getters() {
    for get in 0..5 {
        let mut u = rel("ftp://h:21/f;type=a");
        let _ = match get {
            0 => u.user().to_vec(),
            1 => u.password().to_vec(),
            2 => u.params().to_vec(),
            3 => u.query().to_vec(),
            _ => u.fragment().to_vec(),
        };
        assert!(u.is_parsed(), "getter {get}");
        assert_eq!(u.port(), 21);
    }
    let mut u = rel("http://h/p;a?b#c");
    assert_eq!(u.params(), b";a");
    assert_eq!(u.query(), b"?b");
    assert_eq!(u.fragment(), b"#c");
    let mut u = rel("ftp://us:pw@h/f");
    assert_eq!(u.user(), b"//us");
    assert_eq!(u.password(), b"pw");
    // A URL that does not parse is reparsed on every call, same result.
    let mut g = RzUrl::new();
    g.set_full_url(b"mailto:x@y", true);
    assert_eq!(g.user(), b"");
    assert!(!g.is_parsed());
    assert_eq!(g.scheme(), b"mailto");
    assert_eq!(g.url_type(), 0);
}

#[test]
fn get_port_and_full_url_not_lazy() {
    let mut u = rel("http://h:8080/x");
    assert_eq!(u.port(), 0);
    assert_eq!(u.full_url(), b"http://h:8080/x");
    assert!(!u.is_parsed());
    assert_eq!(u.server(), b"h");
    assert_eq!(u.port(), 8080);
}

#[test]
fn port_digit_run_wraps() {
    for (p, want) in [
        ("4294967296", 0u32),
        ("4294967297", 1),
        ("99999", 99999),
        ("0000000000000001", 0),
        ("1234567890123456789", 2_249_056_121),
    ] {
        let mut u = RzUrl::from_text(format!("http://h:{p}/z").as_bytes());
        assert_eq!(u.server(), b"h");
        assert_eq!(u.port(), want, "{p}");
        assert_eq!(u.absolute_path(), b"/z", "{p}");
    }
    assert_eq!(atol_digits(b"123456789012345"), 123_456_789_012_345u64 as u32);
}

#[test]
fn unknown_scheme_kept() {
    let mut u = rel("gopher://h/x");
    assert_eq!(u.url_type(), RELATIVE);
    assert_eq!(u.scheme(), b"gopher");
    assert_eq!(u.server(), b"h");
    assert_eq!(u.absolute_path(), b"/x");
    let mut g = RzUrl::new();
    g.set_full_url(b"gopher://h/x", true);
    assert_eq!(g.url_type(), NONE);
    assert!(!g.is_parsed());
    // ChangeURL treats it, type-1 schemes and `mailto:` as "N has a scheme".
    for new in [&b"gopher://h/x"[..], b"about:blank", b"mailto:a@b"] {
        let mut c = RzUrl::new();
        assert!(c.change_url(new, b"http://a/b"));
        assert_eq!(c.full_url(), new);
        assert!(!c.is_parsed());
        assert_eq!(c.stored_url_type(), 0);
    }
}

#[test]
fn setters_are_plain_copies() {
    let mut u = rel("http://h:81/p");
    assert_eq!(u.url_type(), HTTP);
    u.set_scheme(b"ftp");
    u.set_user(b"me");
    u.set_password(b"pw");
    u.set_server(b"s");
    u.set_path(b"/q");
    u.set_params(b"p");
    u.set_query(b"?x");
    u.set_fragment(b"#y");
    assert!(u.is_parsed());
    assert_eq!(u.stored_url_type(), HTTP);
    assert_eq!(u.port(), 81);
    assert_eq!(u.full_url(), b"http://h:81/p");
    assert_eq!(
        u.stored_components(),
        [&b"ftp"[..], b"me", b"pw", b"s", b"/q", b"p", b"?x", b"#y"]
    );
    assert_eq!(u.make_url_string_from_components(), b"ftp://me:pw@s:81/qp?x#y");
    // On an unparsed URL the next lazy getter reparses and drops the setter values.
    let mut v = rel("http://h/p");
    v.set_server(b"other");
    assert_eq!(v.server(), b"h");
}

#[test]
fn set_full_url_invalidate() {
    let mut u = rel("http://h:81/p");
    assert_eq!(u.server(), b"h");
    u.set_full_url(b"http://k/q", false);
    assert!(u.is_parsed());
    assert_eq!(u.full_url(), b"http://k/q");
    assert_eq!(u.server(), b"h");
    assert_eq!(u.port(), 81);
    u.set_full_url(b"http://k/q", true);
    assert!(!u.is_parsed());
    assert_eq!(u.port(), 81);
    assert_eq!(u.stored_url_type(), HTTP);
    assert_eq!(u.server(), b"k");
    assert_eq!(u.port(), 0);
}

#[test]
fn change_url_early_returns_keep_stale_fields() {
    let mut u = rel("http://us:pw@h:81/p;a?b#c");
    assert_eq!(u.url_type(), HTTP);
    // New empty: full = base, unparsed; user/params/query/fragment stay.
    assert!(u.change_url(b"", b"www.x.com/y"));
    assert_eq!(u.full_url(), b"www.x.com/y");
    assert!(!u.is_parsed());
    assert_eq!(u.port(), 0);
    assert_eq!(
        u.stored_components(),
        [&b""[..], b"//us", b"", b"", b"", b";a", b"?b", b"#c"]
    );
    // allowRelative (true here) is left unchanged: relative parse, not guess mode.
    assert_eq!(u.url_type(), RELATIVE);

    let mut g = RzUrl::new();
    assert!(g.change_url(b"", b"www.x.com/y"));
    assert!(!g.allow_relative());
    assert_eq!(g.url_type(), HTTP);
    assert_eq!(g.server(), b"www.x.com");

    // Base empty parses with this object's own allowRelative.
    let mut g = RzUrl::new();
    assert!(g.change_url(b"www.x.com/y", b""));
    assert_eq!(g.stored_url_type(), HTTP);
    assert!(!g.allow_relative());
    let mut r = rel("");
    assert!(r.change_url(b"x/y", b""));
    assert_eq!(r.stored_url_type(), RELATIVE);
    // Null base is empty.
    let mut n = RzUrl::new();
    assert!(!n.change_url_opt(b"x/y", None));
    assert_eq!(n.full_url(), b"x/y");
}

#[test]
fn change_url_base_object_overload() {
    let mut u = RzUrl::new();
    assert!(!u.change_url_with_base(b"x/y", None));
    assert_eq!(u.full_url(), b"x/y");
    // A base with components but an empty stored full text counts as null.
    let mut b = RzUrl::new();
    b.set_scheme(b"http");
    b.set_server(b"a");
    let mut u = RzUrl::new();
    assert!(!u.change_url_with_base(b"x/y", Some(&b)));
    // Otherwise the stored full text is used as is (not rebuilt).
    let mut b = rel("http://a/d/e");
    b.set_server(b"zzz");
    let mut u = RzUrl::new();
    assert!(u.change_url_with_base(b"x", Some(&b)));
    assert_eq!(u.full_url(), b"http://a/d/x");
}

#[test]
fn assignment_copies_verbatim() {
    let mut u = rel("http://h:81/p?q");
    let _ = u.server();
    u.set_full_url(b"other", false);
    let c = u.clone();
    assert_eq!(c, u);
    assert!(c.is_parsed() && c.allow_relative());
    assert_eq!(c.port(), 81);
    assert_eq!(c.full_url(), b"other");
}

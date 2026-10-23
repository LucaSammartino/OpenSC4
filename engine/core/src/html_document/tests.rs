//! Conformance tests from the worked examples of docs/specs/html-document.md.

use super::*;

#[derive(Default)]
struct TestHost {
    downloads: Vec<Vec<u8>>,
    delivered: Vec<(Option<NodeId>, ImageHandle)>,
    written: Vec<(Vec<u8>, Vec<u8>)>,
    removed: Vec<Vec<u8>>,
    loaded: Vec<Vec<u8>>,
    surfaces: u64,
    visited: bool,
    retained: Vec<bool>,
    document_refs: u32,
    releases: u32,
    withdrawn: Vec<(Vec<u8>, u32)>,
    released_images: Vec<ImageHandle>,
    forwarded: Vec<Vec<u8>>,
    rgb_calls: u32,
    fail_loader: bool,
    fail_load: bool,
    fail_write: bool,
    reject_image: bool,
    layout: BTreeMap<u32, u32>,
    dbcs: bool,
    fail_surface: bool,
    drop_releases: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl HtmlHost for TestHost {
    fn advance(&self, text: &[u8], i: usize) -> usize {
        (i + if self.dbcs && text[i] == 0x81 { 2 } else { 1 }).min(text.len())
    }
    fn document_layout_word(&self, offset: u32) -> u32 {
        self.layout.get(&offset).copied().unwrap_or(0)
    }
    fn retain_interfaces(&mut self, has_extra: bool) {
        self.retained.push(has_extra);
    }
    fn retain_document(&mut self) {
        self.document_refs += 1;
    }
    fn release_interfaces(&mut self) {
        self.releases += 1;
        self.drop_releases.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    fn withdraw_download(&mut self, url: &[u8], request: u32) {
        self.withdrawn.push((url.to_vec(), request));
    }
    fn release_image(&mut self, image: ImageHandle) {
        self.released_images.push(image);
    }
    fn window_set_base_url(&mut self, url: &[u8]) {
        self.forwarded.push(url.to_vec());
    }
    fn rgb(&mut self, r: i32, g: i32, b: i32) -> u32 {
        self.rgb_calls += 1;
        ((r as u32 & 0xff) << 16) | ((g as u32 & 0xff) << 8) | (b as u32 & 0xff)
    }
    fn set_color(&mut self, value: &[u8], color: &mut u32) {
        *color = match value {
            b"white" => 0xffffff,
            b"#ff0000" => 0xff0000,
            _ => *color,
        };
    }
    fn convert_color(&mut self, parsed: u32) -> u32 {
        parsed | 0x1000_0000
    }
    fn table_bgcolor(&mut self, _value: &[u8]) -> u32 {
        0x123456
    }
    fn is_visited(&mut self, _url: &[u8]) -> bool {
        self.visited
    }
    fn request_download(&mut self, url: &[u8]) -> u32 {
        self.downloads.push(url.to_vec());
        self.downloads.len() as u32
    }
    fn deliver_image(&mut self, waiter: Option<NodeId>, image: ImageHandle) -> bool {
        self.delivered.push((waiter, image));
        !self.reject_image
    }
    fn create_image_loader(&mut self) -> bool {
        !self.fail_loader
    }
    fn load_image(&mut self, path: &[u8]) -> Option<ImageHandle> {
        self.loaded.push(path.to_vec());
        if self.fail_load {
            None
        } else {
            Some(77)
        }
    }
    fn make_temp_path(&mut self) -> Vec<u8> {
        b"tmp0".to_vec()
    }
    fn write_file(&mut self, path: &[u8], data: &[u8]) -> bool {
        if self.fail_write {
            false
        } else {
            self.written.push((path.to_vec(), data.to_vec()));
            true
        }
    }
    fn remove_file(&mut self, path: &[u8]) {
        self.removed.push(path.to_vec());
    }
    fn create_bullet_surface(&mut self, w: u32, h: u32, bg: u32, c: Option<(u32, u32)>) -> Option<ImageHandle> {
        self.surfaces += 1;
        if self.fail_surface {
            return None;
        }
        Some(u64::from(w) << 48 | u64::from(h) << 40 | u64::from(bg & 0xff) << 32 | u64::from(c.map_or(0, |x| x.0)))
    }
}

fn doc() -> HtmlDocument<TestHost> {
    let mut d = HtmlDocument::new(TestHost::default());
    d.init(true, b"http://h/help/");
    d
}

fn parse(src: &str) -> HtmlDocument<TestHost> {
    let mut d = doc();
    d.parse_sgml_elements(src.as_bytes());
    d
}

/// Compact structure dump of the live tree: kinds and text.
fn dump(d: &HtmlDocument<TestHost>, id: NodeId, out: &mut String) {
    let name = match d.kind(id) {
        Kind::Body(_) => "body".to_string(),
        Kind::Text(t) => format!("'{}'", String::from_utf8_lossy(&t.text)),
        Kind::Container(k) => format!("c{k}"),
        Kind::Heading(l) => format!("h{}", l + 1),
        Kind::List { kind, .. } => format!("list{kind}"),
        Kind::Table(_) => "table".into(),
        Kind::Tr(_) => "tr".into(),
        Kind::Td(_) => "td".into(),
        Kind::Img(_) => "img".into(),
        k => format!("{k:?}")
            .split(['(', ' ', '{'])
            .next()
            .unwrap_or("")
            .to_lowercase(),
    };
    out.push_str(&name);
    let kids = d.children(id);
    if !kids.is_empty() {
        out.push('[');
        for (i, &k) in kids.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            dump(d, k, out);
        }
        out.push(']');
    }
}

fn tree(src: &str) -> String {
    let d = parse(src);
    let mut s = String::new();
    dump(&d, d.root.expect("root"), &mut s);
    s
}

fn texts(d: &HtmlDocument<TestHost>) -> Vec<(String, Style, u32)> {
    let mut v = Vec::new();
    fn walk(d: &HtmlDocument<TestHost>, id: NodeId, v: &mut Vec<(String, Style, u32)>) {
        if let Kind::Text(t) = d.kind(id) {
            v.push((String::from_utf8_lossy(&t.text).into_owned(), t.style.clone(), t.align));
        }
        for &k in d.children(id) {
            walk(d, k, v);
        }
    }
    walk(d, d.root.unwrap(), &mut v);
    v
}

#[test]
fn constructor_and_font_table() {
    let mut d = HtmlDocument::new(TestHost::default());
    assert_eq!(d.font_sizes, DEFAULT_FONT_SIZES);
    assert_eq!((d.proportional_size, d.fixed_size), (14, 14));
    assert_eq!(d.proportional_face, b"arial");
    assert_eq!(d.fixed_face, b"courier new");
    d.set_font_size_table(&[6, 7, 8]);
    assert_eq!(d.font_sizes, [6, 7, 8, 14, 18, 24, 36]);
    assert_eq!(d.proportional_size, 14);
    let mut out = [9; 9];
    assert_eq!(d.get_font_size_table(&mut out), 7);
    assert_eq!(out, [6, 7, 8, 14, 18, 24, 36, 0, 0]);
    assert!(d.init(false, b""));
    assert_eq!((d.style.selector, d.style.size, d.style.color), (1, 14, 0));
    d.style.size = 3;
    assert!(d.init(true, b"x"));
    assert_eq!(d.style.size, 3);
    assert_eq!(
        HtmlDocument::<TestHost>::query_interface(0x452294aa),
        Some(Base::Secondary)
    );
    assert_eq!(
        HtmlDocument::<TestHost>::query_interface(0xe0eca23a),
        Some(Base::Primary)
    );
    assert_eq!(HtmlDocument::<TestHost>::query_interface(7), None);
}

#[test]
fn tag_table_and_sets() {
    assert_eq!(TAGS.len(), 160);
    assert_eq!(tag_id(b"/NOBR"), Some(162));
    assert_eq!(tag_id(b"/IMG"), None);
    assert!(sets::flow(3) && sets::block(126) && sets::block(89) && !sets::flow(149));
}

#[test]
fn tokens_and_attributes() {
    let mut d = doc();
    d.source = b"<a  href=\"x.html\" >".to_vec();
    d.get_token(false);
    assert_eq!((d.token, d.tag_text.as_slice()), (54, &b"A"[..]));
    assert_eq!(d.find_attribute("HREF", false), Some(b"x.html".to_vec()));
    d.source = b"<!-- c --><b>".to_vec();
    d.pos = 0;
    d.get_token(false);
    assert_eq!(d.token, 22);
    d.source = b"<foo bar>".to_vec();
    d.pos = 0;
    d.get_token(false);
    assert_eq!(d.token, 0);
    assert_eq!(d.attrs, vec![(b"BAR".to_vec(), vec![])]);
    d.source = b"<td HREF=foo.htm target='_x' href=\"bar\" NOWRAP>".to_vec();
    d.pos = 0;
    d.get_token(false);
    assert_eq!(
        d.attrs,
        vec![
            (b"HREF".to_vec(), b"foo.htm".to_vec()),
            (b"TARGET".to_vec(), b"_x".to_vec()),
            (b"NOWRAP".to_vec(), vec![])
        ]
    );
    d.source = b"<td A=1 \" =\"x\" B>".to_vec();
    d.pos = 0;
    d.get_token(false);
    assert_eq!(d.attrs, vec![(b"A".to_vec(), b"1".to_vec()), (b"B".to_vec(), vec![])]);
    d.source = b"<td a= b>".to_vec();
    d.pos = 0;
    d.get_token(false);
    assert_eq!(d.find_attribute("A", true), Some(b" b".to_vec()));
    assert_eq!(d.find_attribute("A", true), None);
    // Unterminated tag: token 0, position just after '<'.
    d.source = b"ab<c".to_vec();
    d.pos = 2;
    d.get_token(false);
    assert_eq!((d.token, d.pos), (0, 3));
    // Comments join text.
    d.source = b"a<!--x-->b<!-- open".to_vec();
    d.pos = 0;
    d.get_token(false);
    assert_eq!((d.token, d.pending.as_slice()), (3, &b"ab"[..]));
}

#[test]
fn entry_points() {
    assert_eq!(tree("plain  text"), "body['plain  text']");
    let d = parse("plain\r\ntext");
    let t = texts(&d);
    assert!(t[0].1.pre);
    assert_eq!(
        normalize_segment_text(&d.host, b"a  \t\r\n b", false),
        b" a b"[1..].to_vec()
    );
    assert_eq!(
        normalize_segment_text(&d.host, b"a  \t\r\n b", true),
        b"a  \t\n b".to_vec()
    );
    assert_eq!(normalize_segment_text(&d.host, b"x\r\ry", true), b"x\n\ny".to_vec());
    assert_eq!(tree("<html><body>Hi</body></html>"), "body['Hi']");
    assert_eq!(
        tree("<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01//EN\"><html><p>x</html>"),
        "body[c21['x']]"
    );
    let mut d = doc();
    assert!(!d.parse_sgml_elements(b"<!DOCTYPE svg><html>x</html>"));
    assert!(d.root.is_none());
}

#[test]
fn head_elements() {
    let d = parse(
        "<html><head><title>T 1</title><base href=\"http://x/y/\"><meta name=n content=c>\
         <meta name=z><link href=a.css rev=r title=t><style>p{}</style></head><body>b</body></html>",
    );
    assert_eq!(d.title, b"T 1");
    assert_eq!(d.base_url.as_deref(), Some(&b"http://x/y/"[..]));
    assert_eq!(d.metas.len(), 1);
    assert_eq!(d.metas[0].content, b"c");
    assert_eq!(d.links[0].href, b"a.css");
    assert!(!d.links[0].forward);
    assert_eq!(d.links[0].title.as_deref(), Some(&b"t"[..]));
    assert_eq!(texts(&d)[0].0, "b");
}

#[test]
fn body_attributes() {
    let d = parse("<html><body bgcolor=white text=#ff0000 link=white bgproperties>Hi</body></html>");
    let b = d.body().unwrap();
    assert_eq!(b.bgcolor, Some(0xffffff));
    assert_eq!(b.link, 0x10ff_ffff);
    assert!(b.bgproperties);
    assert_eq!(texts(&d)[0].1.color, 0x10ff_0000);
    assert!(!d.pushback || d.token != tok::BODY_END);
}

#[test]
fn forms_select_and_misnesting() {
    assert_eq!(tree("<html><form>a<p>b</p></form></html>"), "body[form['a',c21['b']]]");
    assert_eq!(tree("<html><form></form>z</html>"), "body['z']");
    assert_eq!(tree("<html><form>a</div>b</form>c</html>"), "body[form['a'],'c']");
    assert_eq!(
        tree("<html><select><option>One<option>Two</select></html>"),
        "body[select[optionitem['One'],optionitem['Two']]]"
    );
    assert_eq!(tree("<html><select><option></option></select>q</html>"), "body['q']");
    assert_eq!(tree("<html>x<sup>2</sup>y</html>"), "body['x','y']");
    let d = parse("<html><b>x</i>y</b>z</html>");
    let t = texts(&d);
    assert_eq!(t.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["x", "z"]);
    assert_eq!((t[0].1.flags, t[1].1.flags), (BOLD, 0));
    let d = parse("<html><em>a<p>b</p></em>c</html>");
    let t = texts(&d);
    assert_eq!(
        t.iter().map(|x| (x.0.as_str(), x.1.flags)).collect::<Vec<_>>(),
        [("a", ITALIC), ("c", 0)]
    );
}

#[test]
fn blocks() {
    assert_eq!(tree("<html><p>a<p>b</html>"), "body[c21['a'],c21['b']]");
    assert_eq!(tree("<html><dl><dt>t<dd>d</dl></html>"), "body[c8[c9['t'],c10['d']]]");
    assert_eq!(tree("<html><blockquote>q</blockquote></html>"), "body[blockquote['q']]");
    assert_eq!(tree("<html><center>c</center></html>"), "body[div['c']]");
    let d = parse("<html><center>c</center><div align=right>r</div></html>");
    assert_eq!(texts(&d).iter().map(|t| t.2).collect::<Vec<_>>(), [1, 2]);
    let d = parse("<html><h2>Title</h2><i><h1>x</h1></i></html>");
    let t = texts(&d);
    assert_eq!((t[0].1.size, t[0].1.flags, t[0].1.selector), (32, BOLD, 9));
    assert_eq!((t[1].1.size, t[1].1.flags), (40, BOLD));
    let d = parse("<html><pre>a\n b</pre>c</html>");
    let t = texts(&d);
    assert!(t[0].1.pre && t[0].1.selector == 2 && !t[1].1.pre);
    let d = parse("<html><hr width=50% size=2><hr width=300 align=top></html>");
    let hrs: Vec<Hr> = d
        .children(d.root.unwrap())
        .iter()
        .filter_map(|&k| match d.kind(k) {
            Kind::Hr(h) => Some(h.clone()),
            _ => None,
        })
        .collect();
    assert_eq!((hrs[0].width, hrs[0].percent, hrs[0].size), (Some(50), true, Some(2)));
    assert_eq!(
        (hrs[1].width, hrs[1].percent, hrs[1].align),
        (Some(300), false, Some(2))
    );
}

#[test]
fn lists_and_bullets() {
    let d = parse("<html><ol><li>a<li>b</ol><ol><li>c</ol><ul><li>u</ul></html>");
    let t = texts(&d);
    assert_eq!(
        t.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
        ["  1. ", "a", "  2. ", "b", "  1. ", "c", "u"]
    );
    let lists: Vec<Kind> = d.children(d.root.unwrap()).iter().map(|&k| d.kind(k).clone()).collect();
    assert_eq!(lists[1], Kind::List { kind: 7, serial: 1 });
    assert_eq!(lists[2], Kind::List { kind: 6, serial: 0 });
    let ul = d.children(d.root.unwrap())[2];
    let li = d.children(ul)[0];
    let kids = d.children(li);
    match d.kind(kids[0]) {
        Kind::Img(i) => assert_eq!(
            i.surface.map(|s| (s >> 48, (s >> 40) & 0xff, s & 0xff)),
            Some((11, 9, 7))
        ),
        k => panic!("{k:?}"),
    }
    assert_eq!(*d.kind(kids[1]), Kind::HangingIndent);
    assert_eq!(format!("{:3}. ", 1000), "1000. ");
}

#[test]
fn tables() {
    let src = "<html><table border=1><tr><td align=right>1</td><td>2</td></tr></table></html>";
    assert_eq!(tree(src), "body[table[tr[td['1'],td['2']]]]");
    let d = parse(src);
    assert_eq!(texts(&d).iter().map(|t| t.2).collect::<Vec<_>>(), [2, 0]);
    assert_eq!(
        tree("<html><table><tr><td>a<td>b</tr></table></html>"),
        "body[table[tr[td['a'],td['b']]]]"
    );
    assert_eq!(
        tree("<html><table><colgroup><col></colgroup><tr><td>x</td></tr></table></html>"),
        "body[table]"
    );
    assert_eq!(
        tree("<html><table><colgroup><col><tr><td>x</td></tr></table></html>"),
        "body[table[tr[td['x']]]]"
    );
    assert_eq!(tree("<html><table><tr></tr></table></html>"), "body[table]");
    assert_eq!(
        tree("<html><table><tr><td>x</table></html>"),
        "body[table[tr[td['x']]]]"
    );
    assert_eq!(tree("<html><table><tr><td>x</td></tr>"), "body");
    assert_eq!(
        tree("<html><table><thead><tr><th>h</tr></thead><tbody><tr><td>b</tbody></table></html>"),
        "body[table[tr[th['h']],tr[td['b']]]]"
    );
}

#[test]
fn inline_elements() {
    let d = parse("<html><a href=\"faq.htm\">FAQ</a><a name=top>T</a></html>");
    let root = d.root.unwrap();
    match d.kind(d.children(root)[0]) {
        Kind::Anchor(a) => assert_eq!(a.href_absolute.as_deref(), Some(&b"http://h/help/faq.htm"[..])),
        k => panic!("{k:?}"),
    }
    let t = texts(&d);
    assert_eq!((t[0].1.color, t[0].1.flags, t[0].1.link), (0xff, UNDERLINE, true));
    assert_eq!((t[1].1.flags, t[1].1.link), (0, true));
    let mut d = d;
    assert_eq!(d.find_named_anchor(b"top"), Some(d.children(root)[1]));
    let d = parse("<html><span style=\"color:#ff0000; text-decoration:underline\">s</span></html>");
    assert_eq!((texts(&d)[0].1.color, texts(&d)[0].1.flags), (0x00ff_0000, UNDERLINE));
    let d = parse("<html><span style=\"color: rgb(1,2,3);font-family:Verdana;font-size:20px\">s</span></html>");
    let s = &texts(&d)[0].1;
    assert_eq!(
        (s.color, s.face.as_slice(), s.size, s.selector),
        (0x010203, &b"verdana"[..], 14, 0)
    );
    let d = parse("<html><font size=+1 color=white face=Times>a</font><font size=-9>b</font><font size=3>c</font><big>d</big><tt>e</tt></html>");
    let t = texts(&d);
    assert_eq!(
        (t[0].1.size, t[0].1.color, t[0].1.face.as_slice()),
        (18, 0x10ff_ffff, &b"Times"[..])
    );
    assert_eq!(
        (t[1].1.size, t[2].1.size, t[3].1.size, t[4].1.selector),
        (8, 14, 18, 10)
    );
    let d = parse("<html><blink>x</blink></html>");
    assert!(d.has_blink);
    assert_eq!(tree("<html><blink></blink></html>"), "body[blink]");
}

#[test]
fn images_maps_and_downloads() {
    let mut d = parse("<html><img src=a.gif alt=A width=5 align=middle><img src=a.gif><img>z</html>");
    let root = d.root.unwrap();
    let kids = d.children(root).to_vec();
    assert_eq!(kids.len(), 2);
    match d.kind(kids[0]) {
        Kind::Img(i) => assert_eq!(
            (i.url.as_slice(), i.width, i.valign),
            (&b"http://h/help/a.gif"[..], Some(5), Some(3))
        ),
        k => panic!("{k:?}"),
    }
    assert_eq!(d.host.downloads.len(), 1);
    assert_eq!(d.downloads[&b"http://h/help/a.gif"[..]].waiters.len(), 2);
    let rec = DownloadRecord {
        url: b"http://h/help/a.gif".to_vec(),
        is_path: false,
        data: b"GIF89a".to_vec(),
    };
    assert!(d.do_message(MSG_RESOURCE_DOWNLOADED, 1, &rec));
    assert!(!d.do_message(5, 1, &rec));
    assert_eq!(d.host.written, vec![(b"tmp0".to_vec(), b"GIF89a".to_vec())]);
    assert_eq!(d.host.removed, vec![b"tmp0".to_vec()]);
    assert_eq!(d.host.delivered, vec![(Some(kids[0]), 77), (Some(kids[1]), 77)]);
    d.request_image(None, b"http://h/help/a.gif");
    assert_eq!(d.host.delivered.len(), 3);
    let rec2 = DownloadRecord {
        url: b"u".to_vec(),
        is_path: true,
        data: b"c:/f.png".to_vec(),
    };
    d.request_image(None, b"u");
    d.resource_has_been_downloaded(&rec2);
    assert_eq!(d.host.loaded.last().map(Vec::as_slice), Some(&b"c:/f.png"[..]));
    assert_eq!(d.host.removed.len(), 1);
    assert_eq!(
        tree("<html><map name=m><area shape=rect><p>x</p></map></html>"),
        "body[map[area]]"
    );
    // An empty map returns false, which ends ParseBODY (§7.7): the text after it is lost.
    assert_eq!(tree("<html><map name=m></map>y</html>"), "body");
    assert!(d.shutdown());
    assert!(d.root.is_none() && d.downloads.is_empty());
}

#[test]
fn entity_table() {
    assert_eq!(entities::ENTITY_COUNT, 100);
    assert_eq!(get_string_for_escape(b"amp"), "&");
    assert_eq!(get_string_for_escape(b"copy"), "\u{a9}");
    assert_eq!(get_string_for_escape(b"trade"), "?");
    assert_eq!(get_string_for_escape(b"QUOT"), "?");
    assert_eq!(entity_code(b"nbsp"), Some(160));
    assert_eq!(entity_code(b"Eacute"), Some(201));
    assert_eq!(entity_code(b"eacute"), Some(233));
    assert_eq!(entity_code(b"yuml"), Some(255));
}

#[test]
fn alignment_keywords() {
    assert_eq!(
        [b"left".as_ref(), b"Right", b"CENTER", b"top", b"x"].map(make_output_style),
        [0, 2, 1, 0, 0]
    );
    assert_eq!(
        [
            b"left".as_ref(),
            b"right",
            b"center",
            b"top",
            b"middle",
            b"bottom",
            b"x"
        ]
        .map(make_relative_to_text_alignment_style),
        [0, 1, 4, 2, 3, 4, 4]
    );
}

/// Start directly on an element token, so a return value and cursor can be tested without
/// ParseHTMLElements discarding the body's result (§5.5).
fn fragment(src: &str) -> (HtmlDocument<TestHost>, NodeId) {
    let mut d = doc();
    let root = d.new_body();
    d.root = Some(root);
    d.source = src.as_bytes().to_vec();
    d.get_token(false);
    (d, root)
}

#[test]
fn numeric_attribute_conformance() {
    // Every example in §7.11, plus saturation, base-detection and whitespace boundaries.
    for (s, expected) in [
        ("12", 12),
        ("12px", 12),
        ("7g", 7),
        ("1 f", 1),
        ("1a", 26),
        ("12f", 303),
        ("1e3", 483),
        ("ff", 255),
        ("0x10", 16),
        ("xyz", 0),
        ("", 0),
        (" 7", 7),
        ("+5", 5),
        ("-1", u32::MAX),
        ("99999999999", u32::MAX),
        ("-99999999999", 1),
        ("4294967295", u32::MAX),
        ("4294967296", u32::MAX),
        ("0Xfffffffff", u32::MAX),
        (" 0x10", 0),
        ("+ff", 0),
        ("\t\n\r\u{b}\u{c} +12", 12),
        ("0x", 0),
        ("-0", 0),
    ] {
        assert_eq!(convert_to_u32(s.as_bytes()), expected, "{s:?}");
    }
    let d = parse("<html><hr width=1a% size=0x10><img src=x width=-1 height=99999999999></html>");
    let kids = d.children(d.root.unwrap());
    assert!(matches!(
        d.kind(kids[0]),
        Kind::Hr(Hr {
            width: Some(26),
            size: Some(16),
            percent: true,
            ..
        })
    ));
    assert!(matches!(
        d.kind(kids[1]),
        Kind::Img(Img {
            width: Some(u32::MAX),
            height: Some(u32::MAX),
            ..
        })
    ));
}

#[test]
fn font_size_wrapping_and_layout() {
    for (s, expected) in [
        ("4", 18),
        ("9", 36),
        ("+2", 24),
        ("+a", 36),
        ("-5", 8),
        ("-a", 8),
        (" -1", 123),
        ("4294967295", 123),
        ("99999999999", 123),
        ("+-5", 456),
        ("--4", 0), // Open HD7: provisional zero for the first dword outside the object.
        ("2147483648", 8),
        ("3221225472", 8),
        ("+2147483647", 12),
        ("-4294967295", 18),
        ("4294967272", 14),
    ] {
        let (mut d, root) = fragment(&format!("<font size=\"{s}\">x</font>"));
        d.image_loader = true;
        d.host.layout.insert(0x1a4, 123);
        d.host.layout.insert(0x1a0, 456);
        assert!(d.parse_fontstyle(root), "{s}");
        assert_eq!(texts(&d)[0].1.size, expected, "{s}");
        assert_eq!(d.style.size, 14);
    }
    let mut d = doc();
    d.host.layout.insert(0x1a4, 99);
    assert_eq!(d.font_size_word(-1), 0); // not created yet
    d.image_loader = true;
    assert_eq!(d.font_size_word(-1), 99);
    d.set_slot15_byte(0xa5);
    d.pushback = true;
    d.has_blink = true;
    assert_eq!(d.font_size_word(-96), 0x1a501); // offset 0x28
    assert_eq!(d.font_size_word(-24), 14); // current style, not table[0]
    assert_eq!(d.font_size_word(-98), 1); // constructor constant at 0x20
    assert_eq!(d.font_size_word(7), 0); // HD7
}

#[test]
fn repeated_parse_preserves_only_specified_state() {
    let mut d = doc();
    assert!(d.parse_sgml_elements(b"old plain text with a long cursor"));
    assert!(d.parse_sgml_elements(b"<html><body>Hi"));
    assert_eq!(texts(&d)[0].0, "Hi"); // cursor reset
    assert!(d.pushback);
    let old = d.root.unwrap();
    let html = b"<html><b>new</b></html>";
    assert!(d.parse_sgml_elements(html));
    assert_eq!(texts(&d)[0].0.as_bytes(), html); // stale EOF dispatches to plain text
    assert!(!d.is_live(old)); // plain text deletes the previous root
    assert!(!d.pushback);
    assert!(d.parse_sgml_elements(html));
    assert_eq!((texts(&d)[0].0.as_str(), texts(&d)[0].1.flags), ("new", BOLD));
    // HTML parsing overwrites the root without releasing it (§5.5).
    let old = d.root.unwrap();
    d.pushback = false;
    assert!(d.parse_sgml_elements(b"<html><ol><li>x</ol></html>"));
    assert!(d.is_live(old));
    d.pushback = false;
    assert!(d.parse_sgml_elements(b"<html><ol><li>y</ol></html>"));
    assert!(matches!(
        d.kind(d.children(d.root.unwrap())[0]),
        Kind::List { kind: 7, serial: 1 }
    ));
}

#[test]
fn select_and_script_return_values() {
    for (src, expected, kept) in [
        ("<select><option>a</select>", true, true),
        ("<select></select>", true, false),
        ("<select><b>x</b></select>", true, false),
        ("<select><option>a", false, false),
        ("<select><optgroup><option>x</optgroup><option>y</select>", true, true),
        ("<select><optgroup><option>x", false, false),
    ] {
        let (mut d, root) = fragment(src);
        assert_eq!(d.parse_formctrl(root), expected, "{src}");
        assert_eq!(!d.children(root).is_empty(), kept, "{src}");
    }
    for (src, expected) in [("<script>x</script>", true), ("<script>x", false)] {
        let (mut d, root) = fragment(src);
        assert_eq!(d.parse_special(root), expected);
        assert!(d.children(root).is_empty());
    }
}

#[test]
fn lifetime_interfaces_and_shutdown() {
    let mut d = HtmlDocument::new(TestHost::default());
    assert!(d.init(true, b"base"));
    assert!(d.init(false, b"ignored"));
    assert_eq!(d.host.retained, [true]);
    assert_eq!(d.host.forwarded, [b"base".to_vec()]);
    let mut out = Some(Base::Secondary);
    assert!(!d.query_interface_into(17, &mut out));
    assert_eq!(out, Some(Base::Secondary));
    assert_eq!(d.host.document_refs, 0);
    for (iid, base) in [
        (1, Base::Primary),
        (IID_HTML_DOCUMENT, Base::Primary),
        (IID_MESSAGE_TARGET, Base::Secondary),
    ] {
        assert!(d.query_interface_into(iid, &mut out));
        assert_eq!(out, Some(base));
    }
    assert_eq!(d.host.document_refs, 3);
    d.parse_sgml_elements(b"<html><meta content=c><link href=l><p>x</p></html>");
    let root = d.root.unwrap();
    d.request_image(Some(root), b"pending");
    d.request_image(None, b"complete");
    d.resource_has_been_downloaded(&DownloadRecord {
        url: b"complete".to_vec(),
        is_path: true,
        data: b"f".to_vec(),
    });
    assert!(d.shutdown());
    assert!(!d.is_live(root));
    assert!(d.root.is_none() && d.base_url.is_none());
    assert!(d.metas.is_empty() && d.links.is_empty() && d.downloads.is_empty());
    assert!(!d.image_loader && !d.has_extra && !d.initialised);
    assert_eq!(d.host.withdrawn, [(b"pending".to_vec(), 1)]);
    assert_eq!(d.host.released_images, [77]);
    assert_eq!(d.host.releases, 1);
    assert!(d.shutdown());
    assert_eq!(d.host.releases, 1);
    d.init(false, b"again");
    assert_eq!(d.host.retained, [true, false]);
    let next = HtmlDocument::new(TestHost::default());
    assert_eq!(next.host.rgb_calls, 0); // one-time global colours, not once per document
    let releases = d.host.drop_releases.clone();
    drop(d);
    assert_eq!(releases.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[test]
fn download_failures_retries_and_cached_delivery() {
    let mut d = doc();
    let mut image = Some(42);
    d.host.fail_loader = true;
    d.load_from_image_file(b"f", &mut image);
    assert_eq!(image, Some(42));
    assert!(!d.image_loader && d.host.loaded.is_empty());
    let record = DownloadRecord {
        url: b"u".to_vec(),
        is_path: false,
        data: vec![0, 1, 0xff, 0],
    };
    d.resource_has_been_downloaded(&record); // unknown URL: no temp file work
    assert!(d.host.written.is_empty());
    d.request_image(None, b"u");
    d.request_image(Some(9), b"u");
    assert_eq!(d.host.downloads.len(), 1);
    assert!(d.do_message(MSG_RESOURCE_DOWNLOADED, 0, &record));
    assert!(d.host.loaded.is_empty());
    d.host.fail_write = true;
    d.resource_has_been_downloaded(&record);
    assert!(d.host.written.is_empty());
    assert_eq!(d.host.removed.len(), 1); // cleanup even after open/loader failure
    assert_eq!(d.downloads[&b"u"[..]].waiters.len(), 2);
    d.host.fail_loader = false;
    d.host.fail_load = true;
    d.resource_has_been_downloaded(&record);
    assert!(d.image_loader);
    assert_eq!(d.downloads[&b"u"[..]].waiters.len(), 2);
    d.host.fail_load = false;
    d.host.fail_write = false;
    d.resource_has_been_downloaded(&record);
    assert_eq!(d.host.written[0].1, record.data);
    assert_eq!(d.host.delivered, [(None, 77), (Some(9), 77)]);
    assert!(d.downloads[&b"u"[..]].waiters.is_empty());
    d.set_slot15_byte(0xa5);
    d.host.reject_image = true;
    d.request_image(None, b"u");
    assert_eq!(d.slot15_byte, 0xa5);
    d.host.reject_image = false;
    d.request_image(None, b"u");
    assert_eq!(d.slot15_byte, 0);
    assert_eq!(d.host.downloads.len(), 1);
}

#[test]
fn foreign_character_boundaries() {
    let mut d = doc();
    d.host.dbcs = true;
    d.source = b"a\x81<b<b x=\"\x81\"v\" y=\x81 q>".to_vec();
    d.get_token(false);
    assert_eq!(d.pending, b"a\x81<b"); // trail '<' is not a tag
    d.get_token(false);
    assert_eq!(d.token, 22);
    assert_eq!(d.find_attribute("X", true).unwrap(), b"\x81\"v");
    assert_eq!(d.find_attribute("Y", true).unwrap(), b"\x81 q");
    let mut s = b" \t\x81  \t".to_vec();
    d.strip_white_space(&mut s);
    assert_eq!(s, b"\x81 "); // trail space belongs to a non-space character
    d.source = b"<b\x81> x>".to_vec();
    d.pos = 0;
    d.get_token(false);
    assert_eq!(d.pos, d.source.len()); // trail '>' is not the tag delimiter
    assert_eq!(normalize_segment_text(&d.host, b"\x81\r\n", true), b"\x81\r\n");
    let (mut span, root) = fragment("<span>x</span>");
    span.host.dbcs = true;
    span.attrs = vec![(b"STYLE".to_vec(), b"font-family:x\x81;y;color:#ff".to_vec())];
    assert!(span.parse_span(root));
    assert_eq!(texts(&span)[0].1.face, b"x\x81;y");
    assert_eq!(texts(&span)[0].1.color, 255);
    let (mut hr, root) = fragment("<hr>");
    hr.host.dbcs = true;
    hr.attrs = vec![(b"WIDTH".to_vec(), b"1\x81%z".to_vec())];
    assert!(hr.parse_hr(root));
    assert!(matches!(
        hr.kind(hr.children(root)[0]),
        Kind::Hr(Hr {
            width: Some(1),
            percent: false,
            ..
        })
    ));
}

#[test]
fn token_and_entry_boundaries() {
    let mut d = doc();
    d.source = b" \n<html>".to_vec();
    assert!(d.parse_html_document());
    assert_eq!(d.source_text(), b" \n<html>");
    let mut d = doc();
    d.source = b"x<p>".to_vec();
    assert!(!d.parse_html_document());
    assert!(d.root.is_none());
    d.source = b"<b>tail".to_vec();
    d.pos = 0;
    d.token = tok::TEXT;
    d.pushback = true;
    d.get_token(true); // pushed text is skipped, unlike pushed tags
    assert_eq!(d.token, 22);
    let pos = d.pos;
    d.pushback = true;
    d.get_token(true);
    assert_eq!((d.token, d.pos), (22, pos));
    d.source = b"<td width=5><!doctype html>".to_vec();
    d.pos = 0;
    d.get_token(false);
    let attrs = d.attrs.clone();
    d.dump_unhandled_attributes("ignored", "ignored");
    assert_eq!(d.attrs, attrs);
    d.get_token(false);
    assert_eq!(d.attrs, attrs); // DOCTYPE preserves attributes and raw attribute text
    assert_eq!(d.attr_text, b"html");
    d.source = b"<br>".to_vec();
    d.pos = 0;
    d.get_token(false);
    assert!(d.attrs.is_empty());
    let mut d = parse("<html><head><isindex><object>x</object><script>x</script><meta http-equiv=x content=''><link><link href=x rel=next></head>x</html>");
    assert_eq!(d.metas.len(), 1);
    assert_eq!(d.metas[0].http_equiv.as_deref(), Some(&b"x"[..]));
    assert!(d.metas[0].content.is_empty());
    assert_eq!(d.links.len(), 1);
    assert!(d.links[0].forward);
    assert_eq!(texts(&d)[0].0, "x");
    // Open HD8: literal loop consumes /BODY, /HTML and following text up to EOF.
    let (mut body, _) = fragment("<body>a</body>b</html>");
    assert!(body.parse_body());
    assert_eq!(
        texts(&body).iter().map(|t| t.0.as_str()).collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(body.token, tok::EOF);
    assert!(body.pushback);
    d.shutdown();
}

#[test]
fn dispatcher_and_style_conformance() {
    for tag in [
        "textarea", "label", "button", "object", "q", "sub", "sup", "bdo", "noscript", "fieldset", "address",
    ] {
        assert_eq!(
            tree(&format!("<html>a<{tag}>discard</{tag}>b</html>")),
            "body['a','b']",
            "{tag}"
        );
    }
    // DIR/MENU are dispatched by ParseBLOCK even though absent from the BLOCK first set.
    for tag in ["dir", "menu"] {
        let (mut d, root) = fragment(&format!("<{tag}>discard</{tag}>"));
        assert!(d.parse_block(root));
        assert!(d.children(root).is_empty());
    }
    for (tag, flags, selector, size) in [
        ("tt", 0, 10, 14),
        ("i", ITALIC, 1, 14),
        ("b", BOLD, 1, 14),
        ("strike", STRIKE, 1, 14),
        ("big", 0, 1, 18),
        ("small", 0, 1, 12),
        ("u", UNDERLINE, 1, 14),
        ("em", ITALIC, 1, 14),
        ("strong", BOLD, 1, 14),
        ("dfn", ITALIC, 1, 14),
        ("code", 0, 10, 14),
        ("samp", 0, 10, 14),
        ("kbd", 0, 10, 14),
        ("var", 0, 10, 14),
        ("cite", ITALIC, 1, 14),
        ("abbr", 0, 1, 14),
        ("acronym", 0, 1, 14),
    ] {
        let d = parse(&format!("<html><{tag}>x</{tag}>y</html>"));
        let t = texts(&d);
        assert_eq!(
            (t[0].1.flags, t[0].1.selector, t[0].1.size),
            (flags, selector, size),
            "{tag}"
        );
        assert_eq!(t[1].1, doc().style);
    }
    let d = parse("<html><em>a<p>b</p></em>c</html>");
    assert_eq!(texts(&d).iter().map(|t| t.0.as_str()).collect::<Vec<_>>(), ["a", "c"]);
    let (mut d, root) = fragment("<b>x");
    assert!(!d.parse_fontstyle(root));
    assert_eq!(d.style.flags, 0);
    let (mut d, root) = fragment("<blink>x");
    assert!(!d.parse_blink(root));
    assert!(d.has_blink && d.children(root).is_empty());
    for token in [0, 1, 5, 149, 161] {
        d.token = token;
        assert!(!d.parse_flow(root));
        assert!(!d.parse_inline(root));
        assert!(!d.parse_formctrl(root));
        assert!(!d.parse_special(root));
        assert!(!d.parse_block(root));
        assert!(!d.parse_heading(root));
        assert!(!d.parse_list(root));
    }
}

#[test]
fn style_stack_conformance() {
    let mut d = doc();
    let original = d.style.clone();
    d.pop_style();
    assert_eq!(d.style, original);
    d.push_style();
    d.style = Style {
        face: b"custom".to_vec(),
        selector: 0,
        size: 42,
        color: 123,
        flags: 15,
        pre: true,
        link: true,
    };
    let inner = d.style.clone();
    d.push_style();
    d.style.size = 2;
    d.pop_style();
    assert_eq!(d.style, inner);
    d.pop_style();
    assert_eq!(d.style, original);
    d.align = 2;
    d.pop_align();
    assert_eq!(d.align, 2);
    d.push_align(1);
    d.push_align(0);
    d.pop_align();
    assert_eq!(d.align, 1);
    d.pop_align();
    assert_eq!(d.align, 2);
    for size in [0, 256, -1, 12] {
        let (mut d, root) = fragment("<span style='font-size:20px;color:rgb(1,2,3);text-decoration:underline;text-decoration:none;text-decoration:line-through'>x</span>");
        d.style.size = size;
        d.style.link = true;
        assert!(d.parse_span(root));
        let s = &texts(&d)[0].1;
        assert_eq!(s.size, if size == 12 { 12 } else { 14 });
        assert_eq!((s.flags, s.color, s.link), (STRIKE, 0x010203, true));
        assert_eq!(d.style.size, size);
    }
    let (mut d, root) = fragment("<span>x");
    assert!(!d.parse_span(root));
    assert!(d.children(root).is_empty());
}

#[test]
fn table_attribute_and_order_conformance() {
    let d = parse("<html><body><table border=1 width=90% bgcolor=white cellpadding=2 cellspacing=3><caption>gone</caption><thead><tr><th>h</th></tr></thead><tfoot><tr><td>f</td></tr></tfoot><tbody><tr bgcolor=white><td valign=baseline width=12 nowrap align=right>x</td><th align=center>y</th><td>z</td></tr></tbody></table></html>");
    let table = d.children(d.root.unwrap())[0];
    let Kind::Table(t) = d.kind(table) else {
        panic!("table");
    };
    assert_eq!(t.border.as_deref(), Some(&b"1"[..]));
    assert_eq!(t.width.as_deref(), Some(&b"90%"[..]));
    assert_eq!(t.cellpadding.as_deref(), Some(&b"2"[..]));
    assert_eq!(t.cellspacing.as_deref(), Some(&b"3"[..]));
    assert_eq!(t.bgcolor, Some(0x123456));
    assert_eq!(t.rows.len(), 3);
    let Kind::Tr(r) = d.kind(t.rows[2]) else {
        panic!("row");
    };
    assert_eq!(r.cells.len(), 2); // TH is a child, but not in the cell list
    let Kind::Td(c) = d.kind(r.cells[0]) else {
        panic!("cell");
    };
    assert_eq!((c.color, c.valign, c.align), (Some(0x10ffffff), Some(4), 2));
    assert_eq!(c.width.as_deref(), Some(&b"12"[..]));
    assert_eq!(c.nowrap.as_deref(), Some(&b""[..]));
    assert_eq!(
        texts(&d).iter().map(|t| (t.0.as_str(), t.2)).collect::<Vec<_>>(),
        [("h", 0), ("f", 0), ("x", 2), ("y", 0), ("z", 0)]
    );
    assert_eq!(
        tree("<html><table><tr><td>x</tr><thead><tr><td>discard</tr></thead></table></html>"),
        "body[table[tr[td['x']]]]"
    );
    assert_eq!(
        tree("<html><table><tbody><tr><td>x</tr></tbody><tfoot><tr><td>discard</tr></tfoot></table></html>"),
        "body[table[tr[td['x']]]]"
    );
}

#[test]
fn image_map_and_anchor_boundaries() {
    assert_eq!(tree("<html><map><area><p>discard</p></map></html>"), "body[map[area]]");
    assert_eq!(tree("<html><map name=m></map>discard</html>"), "body"); // false stops BODY
    let d = parse("<html><map><p>x</p></map></html>");
    assert_eq!(texts(&d)[0].0, "x");
    for (shape, expected) in [
        ("default", Some(1)),
        ("rect", Some(2)),
        ("circle", Some(3)),
        ("poly", Some(4)),
        ("other", None),
    ] {
        let d = parse(&format!(
            "<html><map><area shape={shape} coords=1 href=x alt=y></map></html>"
        ));
        let map = d.children(d.root.unwrap())[0];
        assert_eq!(d.kind(d.children(map)[0]), &Kind::Area { shape: expected });
    }
    let (mut d, root) = fragment("<a name=lost>x");
    assert!(!d.parse_a(root));
    assert!(d.named_anchors.is_empty());
    let mut d = doc();
    d.host.visited = true;
    d.parse_sgml_elements(b"<html><a name=top href=x>x</a><a name=top>y</a></html>");
    assert_eq!(texts(&d)[0].1.color, 0x6420a4);
    let first = d.children(d.root.unwrap())[0];
    assert_eq!(d.find_named_anchor(b"top"), Some(first));
    assert_eq!(d.find_named_anchor(b"absent"), None);
    let mut d = HtmlDocument::new(TestHost::default());
    d.parse_sgml_elements(b"<html><img src=x></html>");
    let Kind::Img(i) = d.kind(d.children(d.root.unwrap())[0]) else {
        panic!("image");
    };
    assert!(i.url.is_empty());
    assert_eq!(d.host.downloads, [Vec::<u8>::new()]);
}

#[test]
fn bullet_cache_conformance() {
    let mut d = doc();
    let root = d.new_body();
    d.root = Some(root);
    d.style.size = 30;
    d.style.color = 0x55667788; // distinct cache key from other conformance cases
    d.align = 2;
    d.body_mut().unwrap().bgcolor = Some(0xabcdef);
    let first = d.create_bullet(0).unwrap();
    let Kind::Img(i) = d.kind(first) else {
        panic!("bullet");
    };
    let surface = i.surface;
    assert_eq!(i.align, 2);
    assert_eq!(
        surface.map(|s| (s >> 48, (s >> 40) & 0xff, (s >> 32) & 0xff, s & 0xff)),
        Some((19, 17, 0xef, 15))
    );
    d.body_mut().unwrap().bgcolor = Some(0);
    let second = d.create_bullet(0).unwrap();
    assert_eq!(d.host.surfaces, 1); // background is deliberately absent from the cache key
    assert!(matches!(d.kind(second), Kind::Img(i) if i.surface == surface));
    let blank = d.create_bullet(1).unwrap();
    assert_eq!(d.host.surfaces, 2);
    assert!(matches!(d.kind(blank), Kind::Img(i) if i.surface.unwrap() & 0xff == 0));
    d.host.fail_surface = true;
    assert!(d.create_bullet(2).is_none());
}

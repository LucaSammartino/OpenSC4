//! Document entry points, head and element parsers (§4..§7 of docs/specs/html-document.md).

use super::*;

impl<H: HtmlHost> HtmlDocument<H> {
    // ---- entry points (§5) -------------------------------------------------------------

    /// ParseSGMLElements(text) (slot 8).
    pub fn parse_sgml_elements(&mut self, text: &[u8]) -> bool {
        // §5.1 resets the cursor, but deliberately preserves the push-back token and style.
        self.source = text.to_vec();
        self.pos = 0;
        self.parse_cdata();
        self.get_token(false);
        match self.token {
            tok::DOCTYPE => self.parse_sgml_document_type(),
            tok::HTML => self.parse_html_elements(),
            _ => {
                self.pending.clear();
                self.pos = 0;
                self.parse_text_document()
            }
        }
    }

    /// ParseHTMLDocument (slot 10).
    pub fn parse_html_document(&mut self) -> bool {
        self.skip_document_whitespace();
        self.parse_cdata();
        self.get_token(false);
        if self.token == tok::HTML {
            self.parse_html_elements()
        } else {
            false
        }
    }

    /// ParseSGMLDocumentType (slot 9): only the first four attribute characters matter.
    pub fn parse_sgml_document_type(&mut self) -> bool {
        let head: Vec<u8> = self.attr_text.iter().take(4).map(u8::to_ascii_uppercase).collect();
        if head != b"HTML" {
            return false;
        }
        // The DTD version extraction is a local with no lasting effect.
        self.parse_html_document()
    }

    /// ParseTextDocument (slot 11): the whole source as one preformatted text segment.
    pub fn parse_text_document(&mut self) -> bool {
        if let Some(r) = self.root.take() {
            self.delete(r);
        }
        let root = self.new_body();
        self.root = Some(root);
        self.push_style();
        self.style.pre = true;
        let text = self.source.clone();
        self.insert_text_segment(&text, root);
        self.pos = self.source.len();
        self.pop_style();
        true
    }

    /// ParseHTMLElements (§5.5).
    pub fn parse_html_elements(&mut self) -> bool {
        // The previous root is not released here (unlike §5.4).
        let root = self.new_body();
        self.root = Some(root);
        self.skip_document_whitespace();
        if self.token == tok::HTML {
            self.get_token(false);
        }
        self.parse_head();
        self.skip_document_whitespace();
        self.get_token(false);
        self.parse_body();
        self.get_token(true);
        if self.token != tok::HTML_END {
            self.pushback = true;
        }
        true
    }

    // ---- head (§6, §7.1, §7.13) ----------------------------------------------------------

    pub fn parse_head(&mut self) -> bool {
        if self.token == tok::HEAD {
            self.get_token(true);
        }
        loop {
            match self.token {
                15 => {
                    self.parse_meta();
                }
                16 => {
                    self.parse_link();
                }
                17 => {}
                10 => self.parse_title(),
                12 => {
                    self.parse_base();
                }
                13 => {
                    self.skip_until_token(14);
                }
                57 => {
                    self.skip_until_token(58);
                }
                60 => {
                    self.parse_script();
                }
                tok::HEAD_END => return true,
                _ => {
                    self.pushback = true;
                    return true;
                }
            }
            self.get_token(true);
        }
    }

    pub fn parse_title(&mut self) {
        self.get_token(false);
        if self.token == tok::TEXT {
            self.title = self.pending.clone();
        }
        self.get_token(true);
    }

    pub fn parse_base(&mut self) -> bool {
        if let Some(v) = self.find_attribute("HREF", true) {
            self.set_base_url(&v);
        }
        true
    }

    /// ParseSCRIPT: discards the script body. Literal reading: returns the skip result.
    pub fn parse_script(&mut self) -> bool {
        self.skip_until_token(61)
    }

    pub fn parse_meta(&mut self) -> bool {
        let name = self.find_attribute("NAME", true);
        let content = self.find_attribute("CONTENT", true);
        let http_equiv = self.find_attribute("HTTP-EQUIV", true);
        self.dump_unhandled_attributes("ParseMETA", "META");
        match content {
            None => false,
            Some(content) => {
                self.metas.push(MetaRecord {
                    name,
                    content,
                    http_equiv,
                });
                true
            }
        }
    }

    pub fn parse_link(&mut self) -> bool {
        if let Some(href) = self.find_attribute("HREF", true) {
            let href_absolute = self.resolve(&href);
            let forward = self.find_attribute("REV", true).is_none();
            let title = self.find_attribute("TITLE", true);
            self.links.push(LinkRecord {
                title,
                forward,
                href,
                href_absolute,
            });
        }
        true
    }

    // ---- dispatch (§4.2, §7.2, §7.5) -------------------------------------------------------

    pub fn parse_flow(&mut self, parent: NodeId) -> bool {
        if sets::inline(self.token) {
            self.parse_inline(parent)
        } else if sets::block(self.token) {
            self.parse_block(parent)
        } else {
            false
        }
    }

    pub fn parse_inline(&mut self, parent: NodeId) -> bool {
        let t = self.token;
        if t == tok::TEXT {
            let text = self.pending.clone();
            self.insert_text_segment(&text, parent);
            true
        } else if sets::fontstyle(t) {
            self.parse_fontstyle(parent)
        } else if sets::phrase(t) {
            self.parse_phrase(parent)
        } else if sets::special(t) {
            self.parse_special(parent)
        } else if sets::formctrl(t) {
            self.parse_formctrl(parent)
        } else {
            false
        }
    }

    pub fn parse_formctrl(&mut self, parent: NodeId) -> bool {
        match self.token {
            76 => self.parse_input(parent),
            77 => self.parse_select(parent),
            83 => self.skip_until_token(84),
            85 => self.skip_until_token(86),
            87 => self.skip_until_token(88),
            _ => false,
        }
    }

    pub fn parse_special(&mut self, parent: NodeId) -> bool {
        match self.token {
            54 => self.parse_a(parent),
            56 => self.parse_img(parent),
            59 => self.parse_br(parent),
            60 => self.parse_script(),
            62 => self.parse_map(parent),
            70 => self.parse_span(parent),
            74 => self.parse_blink(parent),
            57 | 64 | 66 | 68 | 72 => {
                let close = self.token + 1;
                self.skip_until_token(close)
            }
            _ => false,
        }
    }

    pub fn parse_block(&mut self, parent: NodeId) -> bool {
        let t = self.token;
        match t {
            101 => self.parse_p(parent),
            103 => self.parse_pre(parent),
            109 => self.parse_dl(parent),
            111 => self.parse_div(parent),
            113 => self.skip_until_token(114),
            115 => self.parse_blockquote(parent),
            117 => self.parse_form(parent),
            119 => self.parse_hr(parent),
            120 => self.parse_table(parent),
            122 | 124 | 128 | 130 => self.skip_until_token(t + 1),
            126 => self.parse_div_actual(parent, 127, 1),
            89 | 91 | 93 | 95 | 97 | 99 => self.parse_heading(parent),
            105 | 107 => self.parse_list(parent),
            _ => false,
        }
    }

    // ---- void and form elements (§7.3) -----------------------------------------------------

    pub fn parse_br(&mut self, parent: NodeId) -> bool {
        let id = self.create(Kind::Br);
        self.add(parent, id);
        true
    }

    pub fn parse_input(&mut self, parent: NodeId) -> bool {
        self.dump_unhandled_attributes("ParseINPUT", "INPUT");
        let id = self.create(Kind::Input);
        self.add(parent, id);
        true
    }

    /// ParseSELECT (§7.3): only a missing close tag makes the result false.
    pub fn parse_select(&mut self, parent: NodeId) -> bool {
        let sel = self.create(Kind::Select);
        loop {
            self.get_token(true);
            match self.token {
                79 => {
                    self.parse_option(sel);
                }
                81 => {
                    self.skip_until_token(82);
                }
                _ => break,
            }
        }
        let result = self.token == 78 || self.skip_until_token(78);
        if result && self.has_children(sel) {
            self.add(parent, sel);
        } else {
            self.delete(sel);
        }
        result
    }

    pub fn parse_option(&mut self, select: NodeId) -> bool {
        let opt = self.create(Kind::OptionItem);
        self.get_token(false);
        if self.token == tok::TEXT {
            let text = self.pending.clone();
            self.insert_text_segment(&text, opt);
            self.add(select, opt);
        } else {
            self.delete(opt);
        }
        if self.token != 80 {
            self.pushback = true;
        }
        true
    }

    // ---- DIV / CENTER (§7.4) ---------------------------------------------------------------

    pub fn parse_div(&mut self, parent: NodeId) -> bool {
        let mut a = self.align;
        if let Some(v) = self.find_attribute("ALIGN", true) {
            a = make_output_style(&v);
        }
        self.parse_div_actual(parent, 112, a)
    }

    pub fn parse_div_actual(&mut self, parent: NodeId, close: u32, alignment: u32) -> bool {
        let div = self.create(Kind::Div);
        self.add(parent, div);
        self.push_align(alignment);
        loop {
            self.get_token(false);
            if !sets::flow(self.token) {
                break;
            }
            self.parse_flow(div);
        }
        if self.token != close {
            self.skip_until_token(close);
        }
        self.pop_align();
        true
    }

    // ---- block helpers (§7.6, §7.10, §7.14) ------------------------------------------------

    /// FORM (exported as `ParseP`).
    pub fn parse_form(&mut self, parent: NodeId) -> bool {
        let form = self.create(Kind::Form);
        loop {
            self.get_token(false);
            let t = self.token;
            if t == 60 {
                self.parse_script();
            } else if t == 117 {
                self.skip_until_token(118);
            } else if sets::inline(t) {
                self.parse_inline(form);
            } else if sets::block(t) {
                self.parse_block(form);
            } else {
                break;
            }
        }
        if self.token != 118 {
            self.skip_until_token(118);
        }
        self.keep_if_children(parent, form);
        true
    }

    /// BLOCKQUOTE (exported as `ParseDL`).
    pub fn parse_blockquote(&mut self, parent: NodeId) -> bool {
        let q = self.create(Kind::Blockquote);
        loop {
            self.get_token(false);
            let t = self.token;
            if t == 60 {
                self.parse_script();
            } else if sets::block(t) {
                self.parse_block(q);
            } else if sets::inline(t) {
                self.parse_inline(q);
            } else {
                break;
            }
        }
        if self.token != 116 {
            self.skip_until_token(116);
        }
        self.keep_if_children(parent, q);
        true
    }

    fn keep_if_children(&mut self, parent: NodeId, id: NodeId) -> bool {
        if self.has_children(id) {
            self.add(parent, id);
            true
        } else {
            self.delete(id);
            false
        }
    }

    /// P (0090614f): any non-INLINE token closes it; always kept.
    pub fn parse_p(&mut self, parent: NodeId) -> bool {
        let p = self.create(Kind::Container(0x15));
        self.get_token(false);
        while sets::inline(self.token) {
            self.parse_inline(p);
            self.get_token(false);
        }
        if self.token != 102 {
            self.pushback = true;
        }
        self.add(parent, p);
        true
    }

    /// PRE (00906a5d): no element; preformatted style; the rest up to `</pre>` is dropped.
    pub fn parse_pre(&mut self, parent: NodeId) -> bool {
        self.push_style();
        self.style.selector = 2;
        self.style.pre = true;
        self.get_token(false);
        while sets::inline(self.token) {
            self.parse_inline(parent);
            self.get_token(false);
        }
        if self.token != 104 {
            self.skip_until_token(104);
        }
        self.pop_style();
        true
    }

    /// DL (009066a3).
    pub fn parse_dl(&mut self, parent: NodeId) -> bool {
        let dl = self.create(Kind::Container(8));
        loop {
            self.get_token(true);
            let t = self.token;
            if t == 151 {
                self.parse_dt_dd(dl, 9, 152);
            } else if t == 153 {
                self.parse_dt_dd(dl, 10, 154);
            } else if sets::block(t) {
                self.parse_block(dl);
            } else {
                break;
            }
        }
        let ok = self.token == 110 || self.skip_until_token(110);
        if ok && self.has_children(dl) {
            self.add(parent, dl);
        } else {
            self.delete(dl);
        }
        ok
    }

    /// DT (kind 9, close 152) / DD (kind 10, close 154).
    pub fn parse_dt_dd(&mut self, parent: NodeId, kind: u32, close: u32) -> bool {
        let e = self.create(Kind::Container(kind));
        let mut ok = true;
        self.get_token(false);
        while sets::inline(self.token) {
            ok = self.parse_inline(e);
            self.get_token(false);
            if !ok {
                break;
            }
        }
        if self.token != close {
            self.pushback = true;
        }
        if ok {
            self.add(parent, e);
        } else {
            self.delete(e);
        }
        ok
    }

    /// H1..H6 (00906754).
    pub fn parse_heading(&mut self, parent: NodeId) -> bool {
        let level = match self.token {
            89 => 0u8,
            91 => 1,
            93 => 2,
            95 => 3,
            97 => 4,
            99 => 5,
            _ => return false,
        };
        let close = self.token + 1;
        let h = self.create(Kind::Heading(level));
        self.push_style();
        self.style.selector = 9;
        self.style.size = HEADING_SIZES[level as usize];
        self.style.flags = BOLD;
        self.inline_or_block_loop(h);
        if self.token != close {
            self.skip_until_token(close);
        }
        self.pop_style();
        self.keep_if_children(parent, h);
        true
    }

    /// `GetToken(false)`; INLINE -> ParseINLINE(e), BLOCK -> ParseBLOCK(e), else stop.
    fn inline_or_block_loop(&mut self, e: NodeId) {
        loop {
            self.get_token(false);
            if sets::inline(self.token) {
                self.parse_inline(e);
            } else if sets::block(self.token) {
                self.parse_block(e);
            } else {
                break;
            }
        }
    }

    /// List dispatch (00906f1d), UL (00907d6b) and OL (0090689f).
    pub fn parse_list(&mut self, parent: NodeId) -> bool {
        let (kind, close, serial) = match self.token {
            105 => {
                let s = self.ul_serial;
                self.ul_serial = self.ul_serial.wrapping_add(1);
                (6, 106, s)
            }
            107 => {
                let s = self.ol_serial;
                self.ol_serial = self.ol_serial.wrapping_add(1);
                (7, 108, s)
            }
            _ => return false,
        };
        let list = self.create(Kind::List { kind, serial });
        let mut ok = true;
        let mut number: i32 = if kind == 6 { -1 } else { 1 };
        loop {
            self.get_token(false);
            let t = self.token;
            if t == 149 {
                ok = self.parse_li(list, number);
                if kind == 7 {
                    number = number.wrapping_add(1);
                }
            } else if sets::block(t) {
                ok = self.parse_block(list);
            } else if sets::inline(t) {
                ok = self.parse_inline(list);
            } else {
                break;
            }
        }
        if self.token != close {
            self.skip_until_token(close);
        }
        if ok && self.has_children(list) {
            self.add(parent, list);
            true
        } else {
            self.delete(list);
            false
        }
    }

    /// ParseLI(list, number) (§7.9).
    pub fn parse_li(&mut self, list: NodeId, number: i32) -> bool {
        let item = self.create(Kind::ListItem);
        self.add(list, item);
        if number < 0 {
            if let Some(b) = self.create_bullet(0) {
                self.add(item, b);
            }
        } else {
            let text = format!("{:3}. ", number);
            self.insert_text_segment(text.as_bytes(), item);
        }
        let hi = self.create(Kind::HangingIndent);
        self.add(item, hi);
        self.get_token(false);
        while sets::flow(self.token) {
            self.parse_flow(item);
            self.get_token(false);
        }
        if self.token != 150 {
            self.pushback = true;
        }
        true
    }

    /// HR (§7.10).
    pub fn parse_hr(&mut self, parent: NodeId) -> bool {
        let mut hr = Hr::default();
        if let Some(v) = self.find_attribute("ALIGN", true) {
            hr.align = Some(make_relative_to_text_alignment_style(&v));
        }
        if let Some(v) = self.find_attribute("SIZE", true) {
            hr.size = Some(convert_to_u32(&v));
        }
        if let Some(mut v) = self.find_attribute("WIDTH", true) {
            if let Some(p) = find_character(&self.host, &v, b"%", 0) {
                hr.percent = true;
                v.truncate(p);
            }
            hr.width = Some(convert_to_u32(&v));
        }
        let id = self.create(Kind::Hr(hr));
        self.add(parent, id);
        true
    }

    // ---- body (§7.7) -----------------------------------------------------------------------

    pub fn parse_body(&mut self) -> bool {
        let root = match self.root {
            Some(r) => r,
            None => {
                let r = self.new_body();
                self.root = Some(r);
                r
            }
        };
        if let Some(v) = self.find_attribute("BACKGROUND", true) {
            if let Some(url) = self.resolve(&v) {
                self.body_mut().expect("body").background_url = url;
            }
        }
        if let Some(v) = self.find_attribute("BGCOLOR", true) {
            let mut c = 0xff00_0000;
            self.host.set_color(&v, &mut c);
            self.body_mut().expect("body").bgcolor = Some(c);
        }
        for (name, which) in [("LINK", 0), ("VLINK", 1), ("ALINK", 2)] {
            if let Some(v) = self.find_attribute(name, true) {
                let c = self.shared_color(&v);
                let b = self.body_mut().expect("body");
                match which {
                    0 => b.link = c,
                    1 => b.vlink = c,
                    _ => b.alink = c,
                }
            }
        }
        if let Some(v) = self.find_attribute("TEXT", true) {
            self.style.color = self.shared_color(&v);
            self.style.link = false;
        }
        if self.find_attribute("BGPROPERTIES", true).is_some() {
            self.body_mut().expect("body").bgproperties = true;
        }
        self.dump_unhandled_attributes("ParseBODY", "BODY");
        let bg = self.body().map(|b| b.background_url.clone()).unwrap_or_default();
        if !bg.is_empty() {
            self.request_image(Some(root), &bg);
        }
        if self.token == tok::BODY {
            self.get_token(false);
        }
        let mut result = true;
        loop {
            let t = self.token;
            if sets::block(t) {
                result = self.parse_block(root);
            } else if t == 60 {
                result = self.parse_script();
            } else if sets::inline(t) {
                result = self.parse_inline(root);
            } else if t == tok::EOF {
                break;
            }
            // Literal §7.7 loop: /BODY is skipped like other non-EOF tokens. The worked
            // example instead says it stops there; clarification requested as HD8.
            self.get_token(false);
            if !result {
                break;
            }
        }
        if self.token != tok::BODY_END {
            self.pushback = true;
        }
        result
    }

    /// `cHTMLShared::SetColor` from 0xff000000, then the GS conversion.
    fn shared_color(&mut self, v: &[u8]) -> u32 {
        let mut c = 0xff00_0000;
        self.host.set_color(v, &mut c);
        self.host.convert_color(c)
    }

    // ---- tables (§7.8) ---------------------------------------------------------------------

    pub fn parse_table(&mut self, parent: NodeId) -> bool {
        let mut t = Table {
            border: self.find_attribute("BORDER", true),
            width: self.find_attribute("WIDTH", true),
            ..Table::default()
        };
        if let Some(v) = self.find_attribute("BGCOLOR", true) {
            t.bgcolor = Some(self.host.table_bgcolor(&v));
        }
        t.cellpadding = self.find_attribute("CELLPADDING", true);
        t.cellspacing = self.find_attribute("CELLSPACING", true);
        self.dump_unhandled_attributes("ParseTABLE", "TABLE");
        let table = self.create(Kind::Table(t));
        self.get_token(true);
        if self.token == 132 {
            self.skip_until_token(133);
            self.get_token(true);
        }
        while self.token == 134 || self.token == 135 {
            if self.token == 135 {
                // COLGROUP (00905ea7): push back whatever stops the COL run.
                loop {
                    self.get_token(true);
                    if self.token != 134 {
                        break;
                    }
                }
                self.pushback = true;
            }
            self.get_token(true);
        }
        if self.token == 137 {
            self.parse_row_group(table, 138, false);
            self.get_token(true);
        }
        if self.token == 141 {
            self.parse_row_group(table, 142, false);
            self.get_token(true);
        }
        while self.token == 139 || self.token == 147 {
            self.parse_row_group(table, 140, self.token == 139);
            self.get_token(true);
        }
        let result = self.token == 121 || self.skip_until_token(121);
        if result {
            self.add(parent, table);
        } else {
            self.delete(table);
        }
        result
    }

    /// THEAD / TFOOT / TBODY gap helpers: entered on the group token (`read_first`) or on a
    /// bare TR; TR loop; push-back unless the stop is `close`.
    fn parse_row_group(&mut self, table: NodeId, close: u32, read_first: bool) -> bool {
        // THEAD/TFOOT always read past their own token; TBODY reads once more, a bare TR not.
        if read_first || self.token == 137 || self.token == 141 {
            self.get_token(true);
        }
        while self.token == 147 {
            self.parse_tr(table);
            self.get_token(true);
        }
        if self.token != close {
            self.pushback = true;
        }
        true
    }

    pub fn parse_tr(&mut self, table: NodeId) -> bool {
        let mut tr = Tr::default();
        if let Kind::Table(t) = &self.nodes[table].kind {
            tr.color = t.bgcolor;
        }
        if let Some(v) = self.find_attribute("BGCOLOR", true) {
            tr.color = Some(self.shared_color(&v));
        }
        let row = self.create(Kind::Tr(tr));
        let mut result = true;
        loop {
            self.get_token(true);
            match self.token {
                145 => result = self.parse_th(row),
                143 => result = self.parse_td(row),
                _ => break,
            }
        }
        if self.token != 148 {
            self.pushback = true;
        }
        if !result || !self.has_children(row) {
            self.delete(row);
            return false;
        }
        self.add(table, row);
        if let Kind::Table(t) = &mut self.nodes[table].kind {
            t.rows.push(row);
        }
        true
    }

    pub fn parse_td(&mut self, row: NodeId) -> bool {
        let mut td = Td::default();
        if let Kind::Tr(r) = &self.nodes[row].kind {
            td.color = r.color;
        }
        let cell = self.create(Kind::Td(Td::default()));
        self.add(row, cell);
        if let Kind::Tr(r) = &mut self.nodes[row].kind {
            r.cells.push(cell);
        }
        if let Some(v) = self.find_attribute("VALIGN", true) {
            td.valign = match v.to_ascii_uppercase().as_slice() {
                b"TOP" => Some(1),
                b"MIDDLE" => Some(2),
                b"BOTTOM" => Some(3),
                b"BASELINE" => Some(4),
                _ => None,
            };
        }
        td.width = self.find_attribute("WIDTH", true);
        if let Some(v) = self.find_attribute("BGCOLOR", true) {
            td.color = Some(self.shared_color(&v));
        }
        td.nowrap = self.find_attribute("NOWRAP", true);
        td.align = self.find_attribute("ALIGN", true).map_or(0, |v| make_output_style(&v));
        let align = td.align;
        self.nodes[cell].kind = Kind::Td(td);
        self.push_align(align);
        self.dump_unhandled_attributes("ParseTD", "TD");
        loop {
            self.get_token(false);
            if !sets::flow(self.token) {
                break;
            }
            self.parse_flow(cell);
        }
        if self.token != 144 {
            self.pushback = true;
        }
        self.pop_align();
        true
    }

    pub fn parse_th(&mut self, row: NodeId) -> bool {
        let th = self.create(Kind::Th);
        self.add(row, th);
        self.get_token(false);
        while sets::flow(self.token) {
            self.parse_flow(th);
            self.get_token(false);
        }
        if self.token != 146 {
            self.pushback = true;
        }
        true
    }

    // ---- inline style elements (§7.11) -----------------------------------------------------

    pub fn parse_fontstyle(&mut self, parent: NodeId) -> bool {
        self.push_style();
        let t = self.token;
        let close = t + 1;
        match t {
            18 => self.set_fixed(),
            20 => self.style.flags |= ITALIC,
            22 => self.style.flags |= BOLD,
            24 => self.style.flags |= STRIKE,
            30 => self.style.flags |= UNDERLINE,
            26 => self.style.size = self.font_sizes[4],
            28 => self.style.size = self.font_sizes[2],
            32 => self.font_attributes(),
            _ => {}
        }
        let mut result = true;
        loop {
            self.get_token(false);
            if sets::inline(self.token) {
                self.parse_inline(parent);
            } else if sets::block(self.token) {
                self.parse_block(parent);
            } else {
                break;
            }
        }
        if self.token != close {
            result = self.skip_until_token(close);
        }
        self.pop_style();
        result
    }

    fn set_fixed(&mut self) {
        self.style.face.clear();
        self.style.selector = 10;
    }

    fn font_attributes(&mut self) {
        if let Some(v) = self.find_attribute("SIZE", true) {
            let idx = match v.first() {
                Some(b'+') => (convert_to_u32(&v[1..]).wrapping_add(3) as i32).min(6),
                Some(b'-') => (3u32.wrapping_sub(convert_to_u32(&v[1..])) as i32).max(0),
                _ => (convert_to_u32(&v) as i32).min(6),
            };
            self.style.size = self.font_size_word(idx);
        }
        if let Some(v) = self.find_attribute("COLOR", true) {
            self.style.color = self.shared_color(&v);
            self.style.link = false;
        }
        if let Some(v) = self.find_attribute("FACE", true) {
            self.style.face = v;
            self.style.selector = 0;
        }
        self.dump_unhandled_attributes("ParseFONT", "FONT");
    }

    pub fn parse_phrase(&mut self, parent: NodeId) -> bool {
        self.push_style();
        let t = self.token;
        let close = t + 1;
        match t {
            34 | 38 | 48 => self.style.flags |= ITALIC,
            36 => self.style.flags |= BOLD,
            40 | 42 | 44 | 46 => self.set_fixed(),
            _ => {}
        }
        loop {
            self.get_token(false);
            if !sets::inline(self.token) {
                break;
            }
            self.parse_inline(parent);
        }
        if self.token != close {
            self.skip_until_token(close);
        }
        self.pop_style();
        true
    }

    pub fn parse_span(&mut self, parent: NodeId) -> bool {
        let span = self.create(Kind::Span);
        self.push_style();
        if let Some(css) = self.find_attribute("STYLE", true) {
            self.apply_span_style(&css);
        }
        self.dump_unhandled_attributes("ParseSPAN", "SPAN");
        self.finish_container(parent, span, 71, true)
    }

    /// The content loop, close test and keep/delete shared by SPAN and A.
    fn finish_container(&mut self, parent: NodeId, e: NodeId, close: u32, pop: bool) -> bool {
        self.inline_or_block_loop(e);
        let mut result = true;
        if self.token != close {
            result = self.skip_until_token(close);
        }
        if pop {
            self.pop_style();
        }
        if result {
            self.add(parent, e);
        } else {
            self.delete(e);
        }
        result
    }

    fn apply_span_style(&mut self, css: &[u8]) {
        let mut s = 0;
        loop {
            let colon = match find_character(&self.host, css, b":", s) {
                Some(c) => c,
                None => return,
            };
            let semi = find_character(&self.host, css, b";", colon + 1);
            let mut name = css[s..colon].to_vec();
            let mut value = css[colon + 1..semi.unwrap_or(css.len())].to_vec();
            self.strip_white_space(&mut name);
            self.strip_white_space(&mut value);
            name.make_ascii_lowercase();
            value.make_ascii_lowercase();
            match name.as_slice() {
                b"color" => {
                    if let Some(c) = scan_hex_color(&value) {
                        self.style.color = c;
                    } else if let Some((r, g, b)) = scan_rgb(&value) {
                        self.style.color = self.host.rgb(r, g, b);
                    }
                }
                b"text-decoration" => match value.as_slice() {
                    b"none" => self.style.flags &= !UNDERLINE,
                    b"underline" => self.style.flags |= UNDERLINE,
                    b"line-through" => self.style.flags |= STRIKE,
                    _ => {}
                },
                b"font-family" => {
                    self.style.face = value;
                    self.style.selector = 0;
                }
                b"font-size" => {
                    // The shipping code searches px/pt in the name, so only the reset runs.
                    let sz = self.style.size as u32;
                    if sz == 0 || sz > 255 {
                        self.style.size = 14;
                    }
                }
                _ => {}
            }
            match semi {
                Some(p) if p + 1 < css.len() => s = p + 1,
                _ => return,
            }
        }
    }

    pub fn parse_blink(&mut self, parent: NodeId) -> bool {
        let b = self.create(Kind::Blink);
        self.inline_or_block_loop(b);
        self.has_blink = true;
        if self.token == 75 || self.skip_until_token(75) {
            self.add(parent, b);
            true
        } else {
            self.delete(b);
            false
        }
    }

    pub fn parse_a(&mut self, parent: NodeId) -> bool {
        let mut a = Anchor::default();
        if let Some(v) = self.find_attribute("HREF", true) {
            a.href_absolute = self.resolve(&v);
            a.href = Some(v);
        }
        let named = self.find_attribute("NAME", true);
        let has_name = named.is_some();
        a.name = named;
        let is_link = a.href.is_some();
        let visited = self.has_extra
            && a.href_absolute.as_ref().is_some_and(|u| {
                let u = u.clone();
                self.host.is_visited(&u)
            });
        let anchor = self.create(Kind::Anchor(a));
        self.dump_unhandled_attributes("ParseA", "A");
        self.push_style();
        self.style.link = true;
        if is_link {
            self.style.color = match (self.body(), visited) {
                (Some(b), true) => b.vlink,
                (Some(b), false) => b.link,
                (None, true) => self.default_vlink,
                (None, false) => self.default_link,
            };
            self.style.flags |= UNDERLINE;
        }
        let kept = self.finish_container(parent, anchor, 55, true);
        if kept && has_name {
            // §7.11 explicitly recommends retaining only kept anchors instead of dangling entries.
            self.named_anchors.push(anchor);
        }
        kept
    }

    // ---- images and maps (§7.12) -----------------------------------------------------------

    pub fn parse_img(&mut self, parent: NodeId) -> bool {
        let src = match self.find_attribute("SRC", true) {
            Some(s) => s,
            None => return false,
        };
        let mut img = Img {
            url: self.resolve(&src).unwrap_or_default(),
            ..Img::default()
        };
        img.alt = self.find_attribute("ALT", true);
        img.width = self.find_attribute("WIDTH", true).map(|v| convert_to_u32(&v));
        img.height = self.find_attribute("HEIGHT", true).map(|v| convert_to_u32(&v));
        img.valign = self
            .find_attribute("ALIGN", true)
            .map(|v| make_relative_to_text_alignment_style(&v));
        img.align = self.align;
        self.dump_unhandled_attributes("ParseIMG", "IMG");
        let url = img.url.clone();
        let id = self.create(Kind::Img(img));
        self.add(parent, id);
        self.request_image(Some(id), &url);
        true
    }

    pub fn parse_map(&mut self, parent: NodeId) -> bool {
        let name = self.find_attribute("NAME", true);
        self.dump_unhandled_attributes("ParseMAP", "MAP");
        let map = self.create(Kind::Map { name });
        self.get_token(false);
        let mut result;
        if self.token == 155 {
            result = true;
            while self.token == 155 {
                self.parse_area(map);
                self.get_token(false);
            }
        } else if sets::block(self.token) {
            result = false;
            while sets::block(self.token) {
                result = self.parse_block(map);
                self.get_token(false);
            }
        } else {
            result = false;
        }
        if self.token != 63 {
            self.skip_until_token(63);
        }
        if result {
            self.add(parent, map);
        } else {
            self.delete(map);
        }
        result
    }

    pub fn parse_area(&mut self, map: NodeId) -> bool {
        let shape = self
            .find_attribute("SHAPE", true)
            .and_then(|v| match v.to_ascii_uppercase().as_slice() {
                b"DEFAULT" => Some(1),
                b"RECT" => Some(2),
                b"CIRCLE" => Some(3),
                b"POLY" => Some(4),
                _ => None,
            });
        self.dump_unhandled_attributes("ParseAREA", "AREA");
        let id = self.create(Kind::Area { shape });
        self.add(map, id);
        true
    }
}

/// `sscanf(value, "#%lx")`: whitespace, optional sign, optional 0x, at least one hex digit.
fn scan_hex_color(v: &[u8]) -> Option<u32> {
    let rest = v.strip_prefix(b"#")?;
    let mut i = 0;
    while i < rest.len() && rest[i].is_ascii_whitespace() {
        i += 1;
    }
    let mut neg = false;
    if i < rest.len() && (rest[i] == b'+' || rest[i] == b'-') {
        neg = rest[i] == b'-';
        i += 1;
    }
    if rest.len() >= i + 2
        && rest[i] == b'0'
        && (rest[i + 1] | 0x20) == b'x'
        && rest.get(i + 2).is_some_and(u8::is_ascii_hexdigit)
    {
        i += 2;
    }
    let start = i;
    let mut n: u32 = 0;
    while i < rest.len() && rest[i].is_ascii_hexdigit() {
        n = n
            .wrapping_mul(16)
            .wrapping_add((rest[i] as char).to_digit(16).unwrap_or(0));
        i += 1;
    }
    if i == start {
        return None;
    }
    Some(if neg { n.wrapping_neg() } else { n })
}

/// `sscanf(value, "rgb(%ld,%ld,%ld)") == 3`.
fn scan_rgb(v: &[u8]) -> Option<(i32, i32, i32)> {
    let mut rest = v.strip_prefix(b"rgb(")?;
    let mut out = [0i32; 3];
    for (k, slot) in out.iter_mut().enumerate() {
        let mut i = 0;
        while i < rest.len() && rest[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut neg = false;
        if i < rest.len() && (rest[i] == b'+' || rest[i] == b'-') {
            neg = rest[i] == b'-';
            i += 1;
        }
        let start = i;
        let mut n: i32 = 0;
        while i < rest.len() && rest[i].is_ascii_digit() {
            n = n.wrapping_mul(10).wrapping_add(i32::from(rest[i] - b'0'));
            i += 1;
        }
        if i == start {
            return None;
        }
        *slot = if neg { n.wrapping_neg() } else { n };
        rest = &rest[i..];
        if k < 2 {
            rest = rest.strip_prefix(b",")?;
        }
    }
    Some((out[0], out[1], out[2]))
}

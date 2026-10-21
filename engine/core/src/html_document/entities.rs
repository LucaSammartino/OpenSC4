//! Character entity table of `GetStringForEscape` (docs/specs/html-document.md §10.1).

/// The four markup entities, then the Latin-1 names for codes 160..=255 in order.
const MARKUP: [(&str, u8); 4] = [("quot", 34), ("amp", 38), ("lt", 60), ("gt", 62)];

const LATIN1: [&str; 96] = [
    "nbsp", "iexcl", "cent", "pound", "curren", "yen", "brvbar", "sect", "uml", "copy", "ordf", "laquo", "not", "shy",
    "reg", "macr", "deg", "plusmn", "sup2", "sup3", "acute", "micro", "para", "middot", "cedil", "sup1", "ordm",
    "raquo", "frac14", "frac12", "frac34", "iquest", "Agrave", "Aacute", "Acirc", "Atilde", "Auml", "Aring", "AElig",
    "Ccedil", "Egrave", "Eacute", "Ecirc", "Euml", "Igrave", "Iacute", "Icirc", "Iuml", "ETH", "Ntilde", "Ograve",
    "Oacute", "Ocirc", "Otilde", "Ouml", "times", "Oslash", "Ugrave", "Uacute", "Ucirc", "Uuml", "Yacute", "THORN",
    "szlig", "agrave", "aacute", "acirc", "atilde", "auml", "aring", "aelig", "ccedil", "egrave", "eacute", "ecirc",
    "euml", "igrave", "iacute", "icirc", "iuml", "eth", "ntilde", "ograve", "oacute", "ocirc", "otilde", "ouml",
    "divide", "oslash", "ugrave", "uacute", "ucirc", "uuml", "yacute", "thorn", "yuml",
];

/// Number of entries in the table (100).
pub const ENTITY_COUNT: usize = MARKUP.len() + LATIN1.len();

/// Exact, case-sensitive lookup: the ISO-8859-1 code of `name`, or `None`.
pub fn entity_code(name: &[u8]) -> Option<u8> {
    if let Some(&(_, c)) = MARKUP.iter().find(|(n, _)| n.as_bytes() == name) {
        return Some(c);
    }
    LATIN1.iter().position(|n| n.as_bytes() == name).map(|i| 160 + i as u8)
}

/// `GetStringForEscape(out, name)`: the entity converted from code page 28591 (ISO-8859-1), or
/// `"?"` when the name is unknown. ISO-8859-1 maps byte-for-byte onto U+0000..U+00FF, so the
/// conversion is exact; the caller re-encodes the returned `char` to its own string encoding.
pub fn get_string_for_escape(name: &[u8]) -> String {
    match entity_code(name) {
        Some(c) => char::from(c).to_string(),
        None => "?".to_string(),
    }
}

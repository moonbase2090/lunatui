// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

fn is_ws(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
}

fn skip_ws(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && is_ws(bytes[index]) {
        index += 1;
    }
    index
}

fn tag_name_end(bytes: &[u8], start: usize) -> Option<usize> {
    if !bytes.get(start)?.is_ascii_alphabetic() {
        return None;
    }
    let mut index = start + 1;
    while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'-') {
        index += 1;
    }
    Some(index)
}

pub(super) struct Ends {
    gt: Option<usize>,
    comment: Option<usize>,
    cdata: Option<usize>,
    pi: Option<usize>,
    double: Option<usize>,
    single: Option<usize>,
}

impl Ends {
    pub(super) fn new(text: &str) -> Self {
        Self {
            gt: text.rfind('>'),
            comment: text.rfind("-->"),
            cdata: text.rfind("]]>"),
            pi: text.rfind("?>"),
            double: text.rfind('"'),
            single: text.rfind('\''),
        }
    }
}

fn reaches(last: Option<usize>, from: usize) -> bool {
    last.is_some_and(|at| at >= from)
}

fn attribute_value_end(bytes: &[u8], start: usize, base: usize, ends: &Ends) -> Option<usize> {
    if let quote @ (b'"' | b'\'') = bytes.get(start)? {
        let last = if *quote == b'"' {
            ends.double
        } else {
            ends.single
        };
        if !reaches(last, base + start + 1) {
            return None;
        }
        let close = bytes[start + 1..].iter().position(|b| b == quote)?;
        Some(start + 1 + close + 1)
    } else {
        let mut index = start;
        while index < bytes.len()
            && !is_ws(bytes[index])
            && !matches!(bytes[index], b'"' | b'\'' | b'=' | b'<' | b'>' | b'`')
        {
            index += 1;
        }
        (index > start).then_some(index)
    }
}

fn open_tag_len(bytes: &[u8], base: usize, ends: &Ends) -> Option<usize> {
    let mut index = tag_name_end(bytes, 1)?;
    loop {
        let after_ws = skip_ws(bytes, index);
        let spaced = after_ws > index;
        index = after_ws;
        match bytes.get(index)? {
            b'>' => return Some(index + 1),
            b'/' => {
                return (bytes.get(index + 1) == Some(&b'>')).then_some(index + 2);
            }
            byte if spaced && (byte.is_ascii_alphabetic() || matches!(byte, b'_' | b':')) => {
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric()
                        || matches!(bytes[index], b'_' | b'.' | b':' | b'-'))
                {
                    index += 1;
                }
                let after_name = skip_ws(bytes, index);
                if bytes.get(after_name) == Some(&b'=') {
                    let value_at = skip_ws(bytes, after_name + 1);
                    index = attribute_value_end(bytes, value_at, base, ends)?;
                }
            }
            _ => return None,
        }
    }
}

fn find_from(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

pub(super) fn inline_html_len(text: &str, base: usize, ends: &Ends) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'<') {
        return None;
    }
    match bytes.get(1)? {
        b'!' => {
            if bytes[2..].starts_with(b"--") {
                if bytes[4..].starts_with(b">") {
                    return Some(5);
                }
                if bytes[4..].starts_with(b"->") {
                    return Some(6);
                }
                if !reaches(ends.comment, base + 4) {
                    return None;
                }
                return find_from(bytes, 4, b"-->").map(|at| at + 3);
            }
            if bytes[2..].starts_with(b"[CDATA[") {
                if !reaches(ends.cdata, base + 9) {
                    return None;
                }
                return find_from(bytes, 9, b"]]>").map(|at| at + 3);
            }
            if bytes.get(2)?.is_ascii_alphabetic() {
                if !reaches(ends.gt, base + 3) {
                    return None;
                }
                return find_from(bytes, 3, b">").map(|at| at + 1);
            }
            None
        }
        b'?' => {
            if !reaches(ends.pi, base + 2) {
                return None;
            }
            find_from(bytes, 2, b"?>").map(|at| at + 2)
        }
        b'/' => {
            let end = tag_name_end(bytes, 2)?;
            let close = skip_ws(bytes, end);
            (bytes.get(close) == Some(&b'>')).then_some(close + 1)
        }
        _ => {
            if !reaches(ends.gt, base) {
                return None;
            }
            open_tag_len(bytes, base, ends)
        }
    }
}

fn is_email_local(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&byte)
}

fn email_domain_ok(domain: &[u8]) -> bool {
    if domain.is_empty() {
        return false;
    }
    domain.split(|&byte| byte == b'.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label[0].is_ascii_alphanumeric()
            && label[label.len() - 1].is_ascii_alphanumeric()
            && label
                .iter()
                .all(|&byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

pub(super) fn autolink_len(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'<') {
        return None;
    }
    let close = 1 + bytes[1..].iter().position(|&byte| {
        byte == b'>' || byte == b'<' || byte == b' ' || byte < 0x20 || byte == 0x7f
    })?;
    if bytes[close] != b'>' {
        return None;
    }
    let inner = &bytes[1..close];
    if let Some(colon) = inner.iter().position(|&byte| byte == b':') {
        let scheme = &inner[..colon];
        if (2..=32).contains(&scheme.len())
            && scheme[0].is_ascii_alphabetic()
            && scheme
                .iter()
                .all(|&byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
        {
            return Some(close + 1);
        }
    }
    let at = inner.iter().position(|&byte| byte == b'@')?;
    let local = &inner[..at];
    (!local.is_empty()
        && local.iter().all(|&byte| is_email_local(byte))
        && email_domain_ok(&inner[at + 1..]))
    .then_some(close + 1)
}

const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
];

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len()
        && text.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

fn raw_text_start(text: &str) -> bool {
    ["<script", "<pre", "<style", "<textarea"]
        .iter()
        .any(|name| {
            starts_with_ignore_case(text, name)
                && matches!(
                    text.as_bytes().get(name.len()),
                    None | Some(b' ' | b'\t' | b'>')
                )
        })
}

fn block_tag_start(text: &str) -> bool {
    let body = text[1..].strip_prefix('/').unwrap_or(&text[1..]);
    let name_len = body.bytes().take_while(u8::is_ascii_alphanumeric).count();
    if name_len == 0 {
        return false;
    }
    let name = &body[..name_len];
    let known = BLOCK_TAGS.iter().any(|tag| tag.eq_ignore_ascii_case(name));
    let rest = &body[name_len..];
    known && (rest.is_empty() || rest.starts_with([' ', '\t', '>']) || rest.starts_with("/>"))
}

fn complete_tag_alone(text: &str) -> bool {
    let bytes = text.as_bytes();
    let length = if bytes.get(1) == Some(&b'/') {
        let Some(end) = tag_name_end(bytes, 2) else {
            return false;
        };
        let close = skip_ws(bytes, end);
        if bytes.get(close) != Some(&b'>') {
            return false;
        }
        close + 1
    } else {
        match open_tag_len(bytes, 0, &Ends::new(text)) {
            Some(length) => length,
            None => return false,
        }
    };
    text[length..]
        .bytes()
        .all(|byte| matches!(byte, b' ' | b'\t'))
}

pub(super) fn html_block_start(text: &str, may_interrupt_paragraph: bool) -> Option<u8> {
    if !text.starts_with('<') {
        return None;
    }
    if raw_text_start(text) {
        return Some(1);
    }
    if text.starts_with("<!--") {
        return Some(2);
    }
    if text.starts_with("<?") {
        return Some(3);
    }
    if text.starts_with("<![CDATA[") {
        return Some(5);
    }
    if text.starts_with("<!") && text.as_bytes().get(2).is_some_and(u8::is_ascii_alphabetic) {
        return Some(4);
    }
    if block_tag_start(text) {
        return Some(6);
    }
    let named_raw = {
        let body = text[1..].strip_prefix('/').unwrap_or(&text[1..]);
        ["script", "style", "pre", "textarea"].iter().any(|name| {
            starts_with_ignore_case(body, name)
                && !body
                    .as_bytes()
                    .get(name.len())
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-')
        })
    };
    if may_interrupt_paragraph && complete_tag_alone(text) && !named_raw {
        return Some(7);
    }
    None
}

pub(super) fn html_block_ends(kind: u8, line: &str) -> bool {
    let lower = |needle: &str| {
        line.as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
    };
    match kind {
        1 => lower("</script>") || lower("</pre>") || lower("</style>") || lower("</textarea>"),
        2 => line.contains("-->"),
        3 => line.contains("?>"),
        4 => line.contains('>'),
        5 => line.contains("]]>"),
        _ => false,
    }
}

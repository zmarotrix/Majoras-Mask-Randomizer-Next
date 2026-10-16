//! Prices: the old `MessageCost` table, and rewriting prices in text.

use serde::Deserialize;
use std::sync::OnceLock;

/// One price in the game (`MessageCost`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MessageCost {
    pub name: Option<String>,
    /// Messages showing the price: (message id, which price in the message).
    pub message_ids: Vec<(u16, u8)>,
    pub locations_affected: Vec<String>,
    pub items_affected: Vec<String>,
    pub price_addresses: Vec<u32>,
    pub subtract_price_addresses: Vec<u32>,
    /// `PriceMode` flag this price belongs to.
    pub category: String,
    /// Vanilla price.
    pub cost: u16,
}

pub fn message_costs() -> &'static [MessageCost] {
    static CELL: OnceLock<Vec<MessageCost>> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(include_str!("../../../data/rom/message_costs.json")).expect("bundled price table"))
}

/// .NET's `\w` for the characters messages use.
fn is_word(c: u16) -> bool {
    match c {
        0x30..=0x39 | 0x41..=0x5A | 0x61..=0x7A | 0x5F | 0xAA | 0xB5 | 0xBA => true,
        0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0xFF => true,
        0x00..=0xFF => false,
        c => char::from_u32(c as u32).is_some_and(char::is_alphanumeric),
    }
}

/// Replace the `index`th price ("12 Rupees", `\b[0-9]{1,3}( |\x11|\x10)Rupees?`)
/// in a message with `cost` (the old `WritePrices` regex).
pub fn replace_price(text: &[u16], index: usize, cost: u16, plural: bool) -> Vec<u16> {
    const RUPEE: [u16; 5] = [b'R' as u16, b'u' as u16, b'p' as u16, b'e' as u16, b'e' as u16];
    let is_digit = |c: u16| (0x30..=0x39).contains(&c);
    let mut out = Vec::with_capacity(text.len());
    let mut found = 0usize;
    let mut i = 0usize;
    while i < text.len() {
        let boundary = is_digit(text[i]) && (i == 0 || !is_word(text[i - 1]));
        let mut matched = None;
        if boundary {
            for len in (1..=3).rev() {
                if i + len >= text.len() || !text[i..i + len].iter().all(|&c| is_digit(c)) {
                    continue;
                }
                let sep = text[i + len];
                if !matches!(sep, 0x20 | 0x11 | 0x10) {
                    continue;
                }
                let start = i + len + 1;
                if text.get(start..start + 5) == Some(&RUPEE) {
                    let mut end = start + 5;
                    if text.get(end) == Some(&(b's' as u16)) {
                        end += 1;
                    }
                    matched = Some((end, sep));
                    break;
                }
            }
        }
        match matched {
            Some((end, sep)) => {
                if found == index {
                    out.extend(cost.to_string().encode_utf16());
                    out.push(sep);
                    out.extend("Rupee".encode_utf16());
                    if plural {
                        out.push(b's' as u16);
                    }
                } else {
                    out.extend_from_slice(&text[i..end]);
                }
                found += 1;
                i = end;
            }
            None => {
                out.push(text[i]);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn replaces_the_right_price() {
        assert_eq!(replace_price(&u("It's 10 Rupees, or 20\u{11}Rupees."), 1, 5, true), u("It's 10 Rupees, or 5\u{11}Rupees."));
        assert_eq!(replace_price(&u("1000 Rupees"), 0, 5, true), u("1000 Rupees"));
        assert_eq!(replace_price(&u("x10 Rupees"), 0, 5, true), u("x10 Rupees"));
        assert_eq!(replace_price(&u("1 Rupee!"), 0, 3, true), u("3 Rupees!"));
    }
}

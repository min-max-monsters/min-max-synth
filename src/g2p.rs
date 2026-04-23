//! Rule-based English grapheme-to-phoneme conversion targeting the synth's
//! 36-phoneme set. Good enough for chiptune speech — not aiming for
//! perfection, aiming for fun.
//!
//! Supports an escape hatch: text inside square brackets is parsed as
//! space-separated phoneme labels, e.g. `[AH EH LL]`.

use crate::dsp::{Phoneme, NUM_PHONEMES};

/// Parse a label like "AH" or "EE" into a phoneme index.
fn label_to_index(s: &str) -> Option<usize> {
    let s = s.trim();
    for i in 0..NUM_PHONEMES {
        if Phoneme::from_index(i).label().trim() == s {
            return Some(i);
        }
    }
    // Also accept "_" and "SIL" for silence.
    match s {
        "_" | "SIL" => Some(23),
        _ => None,
    }
}

/// Convert user input text into a sequence of phoneme indices.
///
/// Text inside `[…]` is treated as literal phoneme labels (space-separated).
/// Everything outside brackets is converted with English G2P rules.
/// The result is capped at `max_len` phonemes.
pub fn text_to_phonemes(input: &str, max_len: usize) -> Vec<usize> {
    let mut result = Vec::new();
    let mut rest = input;

    while !rest.is_empty() && result.len() < max_len {
        if let Some(bracket_start) = rest.find('[') {
            // Process text before the bracket.
            let before = &rest[..bracket_start];
            if !before.trim().is_empty() {
                english_to_phonemes(before, &mut result, max_len);
            }
            // Find closing bracket.
            let after_bracket = &rest[bracket_start + 1..];
            if let Some(bracket_end) = after_bracket.find(']') {
                let inside = &after_bracket[..bracket_end];
                for token in inside.split_whitespace() {
                    if result.len() >= max_len {
                        break;
                    }
                    if let Some(idx) = label_to_index(&token.to_ascii_uppercase()) {
                        result.push(idx);
                    }
                }
                rest = &after_bracket[bracket_end + 1..];
            } else {
                // No closing bracket — treat rest as literal phonemes.
                for token in after_bracket.split_whitespace() {
                    if result.len() >= max_len {
                        break;
                    }
                    if let Some(idx) = label_to_index(&token.to_ascii_uppercase()) {
                        result.push(idx);
                    }
                }
                break;
            }
        } else {
            english_to_phonemes(rest, &mut result, max_len);
            break;
        }
    }

    result
}

/// Append phoneme indices for English text (no brackets) into `out`.
fn english_to_phonemes(text: &str, out: &mut Vec<usize>, max_len: usize) {
    let lower = text.to_ascii_lowercase();
    for word in lower.split(|c: char| !c.is_ascii_alphabetic()) {
        if word.is_empty() {
            continue;
        }
        word_to_phonemes(word, out, max_len);
    }
}

// Phoneme index constants matching the Phoneme enum.
const AH: usize = 0;
const EE: usize = 1;
const IH: usize = 2;
const EH: usize = 3;
const AE: usize = 4;
const UH: usize = 5;
const OH: usize = 6;
const OO: usize = 7;
const AW: usize = 8;
const ER: usize = 9;
const MM: usize = 10;
const NN: usize = 11;
const LL: usize = 12;
const RR: usize = 13;
const SS: usize = 14;
const SH: usize = 15;
const FF: usize = 16;
const ZZ: usize = 17;
const VV: usize = 18;
const BB: usize = 19;
const DD: usize = 20;
const GG: usize = 21;
const KK: usize = 22;
const SIL: usize = 23;
const HH: usize = 24;
const TT: usize = 25;
const AY: usize = 26;
const OW: usize = 27;
const EY: usize = 28;
const PP: usize = 29;
const WW: usize = 30;
const YY: usize = 31;
const NG: usize = 32;
const CH: usize = 33;
const TH: usize = 34;
const DH: usize = 35;

/// Convert a single lowercase ASCII word into phoneme indices.
fn word_to_phonemes(word: &str, out: &mut Vec<usize>, max_len: usize) {
    let bytes = word.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len && out.len() < max_len {
        let remaining = len - i;
        let c = bytes[i] as char;
        let next = if i + 1 < len { bytes[i + 1] as char } else { '\0' };
        let prev = if i > 0 { bytes[i - 1] as char } else { '\0' };
        let next2 = if i + 2 < len { bytes[i + 2] as char } else { '\0' };

        // --- Multi-character patterns (longest match first) ---

        // Skip duplicate consonants (ll, ss, tt, etc.)
        if !is_vowel_char(c) && c == next {
            i += 1;
            continue;
        }

        // "tion" / "sion" → SH-UH-NN
        if remaining >= 4 && &word[i..i + 4] == "tion" {
            out.push(SH);
            push_if(out, UH, max_len);
            push_if(out, NN, max_len);
            i += 4;
            continue;
        }
        if remaining >= 4 && &word[i..i + 4] == "sion" {
            out.push(SH);
            push_if(out, UH, max_len);
            push_if(out, NN, max_len);
            i += 4;
            continue;
        }

        // "ough" → AW (rough → UH-FF handled separately)
        if remaining >= 4 && &word[i..i + 4] == "ough" {
            out.push(AW);
            i += 4;
            continue;
        }

        // "igh" → AY
        if remaining >= 3 && &word[i..i + 3] == "igh" {
            out.push(AY);
            i += 3;
            continue;
        }

        // "tch" → CH
        if remaining >= 3 && &word[i..i + 3] == "tch" {
            out.push(CH);
            i += 3;
            continue;
        }

        // "ing" at end → IH-NG
        if remaining == 3 && &word[i..i + 3] == "ing" {
            out.push(IH);
            push_if(out, NG, max_len);
            i += 3;
            continue;
        }

        // "ng" → NG (when not followed by a vowel)
        if remaining >= 2 && c == 'n' && next == 'g'
            && !is_vowel_char(next2)
        {
            out.push(NG);
            i += 2;
            continue;
        }

        // "th" → TH (or DH for common voiced words)
        if remaining >= 2 && c == 't' && next == 'h' {
            // "the", "this", "that", "them", "then", "there", "they" → voiced
            if i == 0 && matches!(&word[..], "the" | "this" | "that" | "them"
                | "then" | "there" | "they" | "those" | "these" | "their"
                | "than" | "though" | "thus")
            {
                out.push(DH);
            } else {
                out.push(TH);
            }
            i += 2;
            continue;
        }

        // "sh" → SH
        if remaining >= 2 && c == 's' && next == 'h' {
            out.push(SH);
            i += 2;
            continue;
        }

        // "ch" → CH
        if remaining >= 2 && c == 'c' && next == 'h' {
            out.push(CH);
            i += 2;
            continue;
        }

        // "wh" → WW
        if remaining >= 2 && c == 'w' && next == 'h' {
            out.push(WW);
            i += 2;
            continue;
        }

        // "ph" → FF
        if remaining >= 2 && c == 'p' && next == 'h' {
            out.push(FF);
            i += 2;
            continue;
        }

        // "ck" → KK
        if remaining >= 2 && c == 'c' && next == 'k' {
            out.push(KK);
            i += 2;
            continue;
        }

        // "qu" → KK-WW
        if remaining >= 2 && c == 'q' && next == 'u' {
            out.push(KK);
            push_if(out, WW, max_len);
            i += 2;
            continue;
        }

        // "aw" → AW ("awesome", "law", "saw")
        if remaining >= 2 && c == 'a' && next == 'w' {
            out.push(AW);
            i += 2;
            continue;
        }

        // "ow" → OW
        if remaining >= 2 && c == 'o' && next == 'w' {
            out.push(OW);
            i += 2;
            continue;
        }

        // "ou" → OW
        if remaining >= 2 && c == 'o' && next == 'u' {
            out.push(OW);
            i += 2;
            continue;
        }

        // "oo" → OO
        if remaining >= 2 && c == 'o' && next == 'o' {
            out.push(OO);
            i += 2;
            continue;
        }

        // "ee" → EE
        if remaining >= 2 && c == 'e' && next == 'e' {
            out.push(EE);
            i += 2;
            continue;
        }

        // "ea" → EE
        if remaining >= 2 && c == 'e' && next == 'a' {
            out.push(EE);
            i += 2;
            continue;
        }

        // "ai" / "ay" → EY
        if remaining >= 2 && c == 'a' && (next == 'i' || next == 'y') {
            out.push(EY);
            i += 2;
            continue;
        }

        // "ey" → EY
        if remaining >= 2 && c == 'e' && next == 'y' {
            out.push(EY);
            i += 2;
            continue;
        }

        // "oi" / "oy" → AW-EE (approximation)
        if remaining >= 2 && c == 'o' && (next == 'i' || next == 'y') {
            out.push(AW);
            push_if(out, EE, max_len);
            i += 2;
            continue;
        }

        // "er" / "ir" / "ur" → ER
        if remaining >= 2 && (c == 'e' || c == 'i' || c == 'u') && next == 'r' {
            out.push(ER);
            i += 2;
            continue;
        }

        // "ar" → AH-RR
        if remaining >= 2 && c == 'a' && next == 'r' {
            out.push(AH);
            push_if(out, RR, max_len);
            i += 2;
            continue;
        }

        // "or" → AW-RR
        if remaining >= 2 && c == 'o' && next == 'r' {
            out.push(AW);
            push_if(out, RR, max_len);
            i += 2;
            continue;
        }

        // --- Single vowels (context-sensitive) ---

        // Silent trailing "e" after consonant: skip.
        // But only if the word has at least one other vowel already.
        if c == 'e' && i == len - 1 && i >= 2 && !is_vowel_char(prev) {
            let has_prior_vowel = word[..i].chars().any(|ch| is_vowel_char(ch) || ch == 'y');
            if has_prior_vowel {
                i += 1;
                continue;
            }
        }

        // Word-final single "e" as the only vowel → EE ("she", "he", "me", "we")
        if c == 'e' && i == len - 1 && i >= 1 {
            let has_prior_vowel = word[..i].chars().any(|ch| is_vowel_char(ch) || ch == 'y');
            if !has_prior_vowel {
                out.push(EE);
                i += 1;
                continue;
            }
        }

        // "a" before consonant + e (magic e): EY
        if c == 'a' && i + 2 < len && !is_vowel_char(next)
            && bytes[i + 2] as char == 'e'
            && (i + 3 == len || !is_vowel_char(bytes[i + 2 + 1] as char))
        {
            out.push(EY);
            i += 1;
            continue;
        }

        // "i" before consonant + e (magic e): AY
        if c == 'i' && i + 2 < len && !is_vowel_char(next)
            && bytes[i + 2] as char == 'e'
            && (i + 3 == len || !is_vowel_char(bytes[i + 2 + 1] as char))
        {
            out.push(AY);
            i += 1;
            continue;
        }

        // "o" before consonant + e (magic e): OH
        if c == 'o' && i + 2 < len && !is_vowel_char(next)
            && bytes[i + 2] as char == 'e'
            && (i + 3 == len || !is_vowel_char(bytes[i + 2 + 1] as char))
        {
            out.push(OH);
            i += 1;
            continue;
        }

        // "u" before consonant + e (magic e): OO
        if c == 'u' && i + 2 < len && !is_vowel_char(next)
            && bytes[i + 2] as char == 'e'
            && (i + 3 == len || !is_vowel_char(bytes[i + 2 + 1] as char))
        {
            out.push(OO);
            i += 1;
            continue;
        }

        // --- Single-character fallbacks ---
        match c {
            'a' => out.push(AE),
            'e' => out.push(EH),
            'i' if i == len - 1 && i > 0 => out.push(AY),  // word-final i = "eye"
            'i' => out.push(IH),
            'o' if i == len - 1 => out.push(OH),  // word-final o = "oh"
            'o' => out.push(AH),
            'u' => out.push(UH),
            'y' if is_consonant_context(prev, next) => out.push(EE),
            'y' => out.push(YY),

            'b' => out.push(BB),
            'c' if next == 'e' || next == 'i' || next == 'y' => out.push(SS),
            'c' => out.push(KK),
            'd' => out.push(DD),
            'f' => out.push(FF),
            'g' if next == 'e' || next == 'i' || next == 'y' => {
                // Soft g (approximate — not always correct).
                out.push(DD);
                // skip the affricate detail for slot economy
            }
            'g' => out.push(GG),
            'h' => out.push(HH),
            'j' => out.push(DD),
            'k' => out.push(KK),
            'l' => out.push(LL),
            'm' => out.push(MM),
            'n' => out.push(NN),
            'p' => out.push(PP),
            'q' => out.push(KK),
            'r' => out.push(RR),
            's' if i == len - 1 && is_voiced_before(prev) => out.push(ZZ),
            's' => out.push(SS),
            't' => out.push(TT),
            'v' => out.push(VV),
            'w' => out.push(WW),
            'x' => {
                out.push(KK);
                push_if(out, SS, max_len);
            }
            'z' => out.push(ZZ),
            _ => {}
        }

        i += 1;
    }
}

#[inline]
fn push_if(out: &mut Vec<usize>, val: usize, max_len: usize) {
    if out.len() < max_len {
        out.push(val);
    }
}

#[inline]
fn is_vowel_char(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

/// "y" acts as a vowel when surrounded by consonants.
#[inline]
fn is_consonant_context(prev: char, next: char) -> bool {
    prev != '\0' && !is_vowel_char(prev) && (next == '\0' || !is_vowel_char(next))
}

/// Letters whose corresponding sounds are voiced (for terminal-s voicing).
#[inline]
fn is_voiced_before(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'b' | 'd' | 'g' | 'l'
        | 'm' | 'n' | 'r' | 'v' | 'w' | 'y' | 'z')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_world() {
        let p = text_to_phonemes("hello world", 16);
        // HH-EH-LL-OH  WW-AW-RR-LL-DD
        assert_eq!(p, vec![HH, EH, LL, OH, WW, AW, RR, LL, DD]);
    }

    #[test]
    fn bracket_escape() {
        let p = text_to_phonemes("[AH EE SS]", 16);
        assert_eq!(p, vec![AH, EE, SS]);
    }

    #[test]
    fn mixed_text_and_brackets() {
        let p = text_to_phonemes("hi [AH] no", 16);
        // HH-AY  AH  NN-OH
        assert_eq!(p, vec![HH, AY, AH, NN, OH]);
    }

    #[test]
    fn max_len_caps() {
        let p = text_to_phonemes("supercalifragilistic", 4);
        assert_eq!(p.len(), 4);
    }

    #[test]
    fn magic_e() {
        // "make" → MM-EY-KK (silent e)
        let p = text_to_phonemes("make", 16);
        assert_eq!(p, vec![MM, EY, KK]);
    }

    #[test]
    fn digraphs() {
        let p = text_to_phonemes("she", 16);
        assert_eq!(p, vec![SH, EE]);

        let p = text_to_phonemes("the", 16);
        assert_eq!(p, vec![DH, EE]);

        let p = text_to_phonemes("church", 16);
        assert_eq!(p, vec![CH, ER, CH]);
    }

    #[test]
    fn single_words() {
        // world = WW-AW-RR-LL-DD (5 phonemes)
        let p = text_to_phonemes("world", 16);
        assert_eq!(p, vec![WW, AW, RR, LL, DD]);

        // hi = HH-AY
        let p = text_to_phonemes("hi", 16);
        assert_eq!(p, vec![HH, AY]);

        // me = MM-EE
        let p = text_to_phonemes("me", 16);
        assert_eq!(p, vec![MM, EE]);
    }

    #[test]
    fn silent_trailing_e() {
        // "name" → NN-EY-MM (silent e)
        let p = text_to_phonemes("name", 16);
        assert_eq!(p, vec![NN, EY, MM]);
    }

    #[test]
    fn aw_digraph() {
        // "awesome" → AW-EH-SS-OH-MM (silent e)
        let p = text_to_phonemes("awesome", 16);
        assert_eq!(p, vec![AW, EH, SS, OH, MM]);
    }
}

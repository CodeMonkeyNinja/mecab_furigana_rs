//! MeCab-based furigana and romaji annotation for Japanese text.
//!
//! Converts Japanese text containing kanji into a bracketed furigana format
//! (`天井[てんじょう]`) and optionally produces Hepburn romaji.  Uses the
//! MeCab morphological analyzer (CLI) under the hood — no Python, no kakasi.
//!
//! # Quick start
//!
//! ```no_run
//! let result = mecab_furigana_rs::annotate("知らない天井だ").unwrap();
//! assert_eq!(result.furigana, "知ら[しら]ない天井[てんじょう]だ");
//! assert!(result.romaji.contains("tenjou"));
//! ```
//!
//! # Prerequisites
//!
//! Install MeCab and a UTF-8 dictionary on the host system:
//!
//! ```bash
//! # Debian / Ubuntu
//! sudo apt install mecab mecab-naist-jdic
//! ```
//!
//! The crate auto-discovers the dictionary from standard system paths.
//!
//! # Platform support
//!
//! Full functionality on Linux.  On other platforms, [`annotate`] returns `None`
//! (the pure conversion helpers like [`kata_to_romaji`] work everywhere).

// ── MeCab dictionary search paths ──────────────────────────────────────────

/// Standard system paths for MeCab UTF-8 dictionaries (checked in order).
/// naist-jdic is richer (person names, neologisms); ipadic-utf8 is the standard IPA dictionary.
const MECAB_DICT_PATHS: &[&str] = &[
    "/usr/share/mecab/dic/naist-jdic",
    "/usr/share/mecab/dic/ipadic-utf8",
    "/usr/lib/mecab/dic/naist-jdic",
    "/usr/lib/mecab/dic/ipadic-utf8",
    "/var/lib/mecab/dic/naist-jdic",
    "/var/lib/mecab/dic/ipadic-utf8",
];

// ── Public types ───────────────────────────────────────────────────────────

/// Result of furigana/romaji annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuriganaResult {
    /// Annotated text with bracketed hiragana readings for kanji.
    /// Example: `"知ら[しら]ない天井[てんじょう]だ"`
    pub furigana: String,
    /// Hepburn romaji derived from MeCab's katakana readings.
    /// Example: `"shira nai tenjou da"`
    pub romaji: String,
}

// ── Public API ─────────────────────────────────────────────────────────────

/// Annotate Japanese text with furigana and romaji via MeCab.
///
/// Returns `None` if MeCab is unavailable or the text produces no output.
/// On non-Linux platforms, always returns `None`.
pub fn annotate(text: &str) -> Option<FuriganaResult> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (furigana, romaji) = mecab_analyze(text)?;
    Some(FuriganaResult { furigana, romaji })
}

/// Annotate text, returning furigana only (skips romaji generation cost).
///
/// The returned `FuriganaResult` will have an empty `romaji` field.
pub fn annotate_furigana_only(text: &str) -> Option<FuriganaResult> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (furigana, _) = mecab_analyze(text)?;
    Some(FuriganaResult { furigana, romaji: String::new() })
}

/// Find the first existing MeCab UTF-8 dictionary directory, or `None`.
pub fn find_mecab_dict() -> Option<&'static str> {
    for &path in MECAB_DICT_PATHS {
        if std::path::Path::new(path).join("sys.dic").exists() {
            return Some(path);
        }
    }
    None
}

// ── Pure helpers (work on all platforms) ────────────────────────────────────

/// True if the string contains any CJK ideograph (kanji).
pub fn has_kanji(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c,
            '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
            | '\u{3400}'..='\u{4DBF}' // Extension A
            | '\u{F900}'..='\u{FAFF}' // Compatibility Ideographs
        )
    })
}

/// Convert katakana string to hiragana (Unicode shift -0x60).
pub fn kata_to_hira(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{30A1}'..='\u{30F6}').contains(&c) {
                char::from_u32(c as u32 - 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// Convert a katakana string to romaji using Hepburn romanization.
///
/// Handles digraphs (キャ→kya), gemination (ッ→double consonant),
/// long vowels (ー→repeat), and loanword kana (ファ→fa, ティ→ti).
pub fn kata_to_romaji(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(chars.len() * 2);
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Small tsu (ッ) → geminate: double the next consonant
        if c == 'ッ' || c == 'っ' {
            if let Some(next_rom) = chars.get(i + 1).and_then(|&nc| single_kana_romaji(nc)) {
                if let Some(first_consonant) = next_rom.chars().next() {
                    if first_consonant != 'a'
                        && first_consonant != 'i'
                        && first_consonant != 'u'
                        && first_consonant != 'e'
                        && first_consonant != 'o'
                    {
                        out.push(first_consonant);
                    }
                }
            }
            i += 1;
            continue;
        }

        // Long vowel mark (ー) → repeat previous vowel
        if c == 'ー' {
            if let Some(last) = out.chars().last() {
                match last {
                    'a' | 'i' | 'u' | 'e' | 'o' => out.push(last),
                    _ => out.push('u'), // default long vowel
                }
            }
            i += 1;
            continue;
        }

        // Try digraph: current + next small kana (ャュョァィゥェォ)
        if i + 1 < chars.len() {
            if let Some(rom) = digraph_romaji(c, chars[i + 1]) {
                out.push_str(rom);
                i += 2;
                continue;
            }
        }

        // Single kana
        if let Some(rom) = single_kana_romaji(c) {
            out.push_str(rom);
        } else {
            // Pass through non-kana (punctuation, latin, etc.)
            out.push(c);
        }
        i += 1;
    }

    out
}

/// Parse raw MeCab stdout into (furigana, romaji).
///
/// Pure function — no I/O.  Useful for testing with synthetic MeCab output
/// without needing the MeCab binary or dictionary installed.
///
/// Each non-EOS line is `surface\tPOS,sub1,sub2,sub3,conj_type,conj_form,base,reading,pronunciation`.
/// Kanji surfaces get bracketed hiragana: `天井[てんじょう]`.
/// Romaji is derived from the katakana reading field.
pub fn parse_mecab_output(stdout: &str) -> Option<(String, String)> {
    let mut furigana = String::new();
    let mut romaji_parts: Vec<String> = Vec::new();

    for line in stdout.lines() {
        if line == "EOS" || line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let surface = parts.next().unwrap_or("");
        let features = parts.next().unwrap_or("");
        let fields: Vec<&str> = features.split(',').collect();

        // Reading is field index 7 (0-based), in katakana.
        let reading_kata = if fields.len() > 7 && fields[7] != "*" {
            fields[7]
        } else {
            ""
        };

        // Furigana: annotate kanji with hiragana reading
        if has_kanji(surface) && !reading_kata.is_empty() {
            let reading_hira = kata_to_hira(reading_kata);
            furigana.push_str(surface);
            furigana.push('[');
            furigana.push_str(&reading_hira);
            furigana.push(']');
        } else {
            furigana.push_str(surface);
        }

        // Romaji: convert katakana reading (or surface if no reading)
        if !reading_kata.is_empty() {
            romaji_parts.push(kata_to_romaji(reading_kata));
        } else {
            // Surface is likely punctuation or already latin
            romaji_parts.push(surface.to_string());
        }
    }

    if furigana.is_empty() {
        None
    } else {
        let romaji = romaji_parts.join(" ")
            .replace("  ", " ")
            .trim()
            .to_string();
        Some((furigana, romaji))
    }
}

// ── MeCab invocation (Linux only) ──────────────────────────────────────────

#[cfg(target_os = "linux")]
fn mecab_analyze(text: &str) -> Option<(String, String)> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let dict = find_mecab_dict()?;
    let mut child = Command::new("mecab")
        .arg("-r")
        .arg("/dev/null")
        .arg("-d")
        .arg(dict)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    child.stdin.take()?.write_all(text.as_bytes()).ok()?;

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_mecab_output(&stdout)
}

#[cfg(not(target_os = "linux"))]
fn mecab_analyze(_text: &str) -> Option<(String, String)> {
    None
}

// ── Kana lookup tables ─────────────────────────────────────────────────────

/// Hepburn romaji for a single katakana/hiragana character.
fn single_kana_romaji(c: char) -> Option<&'static str> {
    Some(match c {
        // Katakana
        'ア' | 'あ' => "a",
        'イ' | 'い' => "i",
        'ウ' | 'う' => "u",
        'エ' | 'え' => "e",
        'オ' | 'お' => "o",
        'カ' | 'か' => "ka",
        'キ' | 'き' => "ki",
        'ク' | 'く' => "ku",
        'ケ' | 'け' => "ke",
        'コ' | 'こ' => "ko",
        'サ' | 'さ' => "sa",
        'シ' | 'し' => "shi",
        'ス' | 'す' => "su",
        'セ' | 'せ' => "se",
        'ソ' | 'そ' => "so",
        'タ' | 'た' => "ta",
        'チ' | 'ち' => "chi",
        'ツ' | 'つ' => "tsu",
        'テ' | 'て' => "te",
        'ト' | 'と' => "to",
        'ナ' | 'な' => "na",
        'ニ' | 'に' => "ni",
        'ヌ' | 'ぬ' => "nu",
        'ネ' | 'ね' => "ne",
        'ノ' | 'の' => "no",
        'ハ' | 'は' => "ha",
        'ヒ' | 'ひ' => "hi",
        'フ' | 'ふ' => "fu",
        'ヘ' | 'へ' => "he",
        'ホ' | 'ほ' => "ho",
        'マ' | 'ま' => "ma",
        'ミ' | 'み' => "mi",
        'ム' | 'む' => "mu",
        'メ' | 'め' => "me",
        'モ' | 'も' => "mo",
        'ヤ' | 'や' => "ya",
        'ユ' | 'ゆ' => "yu",
        'ヨ' | 'よ' => "yo",
        'ラ' | 'ら' => "ra",
        'リ' | 'り' => "ri",
        'ル' | 'る' => "ru",
        'レ' | 'れ' => "re",
        'ロ' | 'ろ' => "ro",
        'ワ' | 'わ' => "wa",
        'ヲ' | 'を' => "wo",
        'ン' | 'ん' => "n",
        // Dakuten
        'ガ' | 'が' => "ga",
        'ギ' | 'ぎ' => "gi",
        'グ' | 'ぐ' => "gu",
        'ゲ' | 'げ' => "ge",
        'ゴ' | 'ご' => "go",
        'ザ' | 'ざ' => "za",
        'ジ' | 'じ' => "ji",
        'ズ' | 'ず' => "zu",
        'ゼ' | 'ぜ' => "ze",
        'ゾ' | 'ぞ' => "zo",
        'ダ' | 'だ' => "da",
        'ヂ' | 'ぢ' => "di",
        'ヅ' | 'づ' => "du",
        'デ' | 'で' => "de",
        'ド' | 'ど' => "do",
        'バ' | 'ば' => "ba",
        'ビ' | 'び' => "bi",
        'ブ' | 'ぶ' => "bu",
        'ベ' | 'べ' => "be",
        'ボ' | 'ぼ' => "bo",
        // Handakuten
        'パ' | 'ぱ' => "pa",
        'ピ' | 'ぴ' => "pi",
        'プ' | 'ぷ' => "pu",
        'ペ' | 'ぺ' => "pe",
        'ポ' | 'ぽ' => "po",
        // Small vowels (used in loanwords: ファ, ティ, etc.)
        'ァ' | 'ぁ' => "a",
        'ィ' | 'ぃ' => "i",
        'ゥ' | 'ぅ' => "u",
        'ェ' | 'ぇ' => "e",
        'ォ' | 'ぉ' => "o",
        'ヴ' => "vu",
        _ => return None,
    })
}

/// Hepburn romaji for two-kana digraphs (e.g. キャ→kya, シュ→shu, チョ→cho).
fn digraph_romaji(first: char, second: char) -> Option<&'static str> {
    // Only match when second char is a small kana (ャュョァィゥェォ)
    if !matches!(
        second,
        'ャ' | 'ュ' | 'ョ' | 'ゃ' | 'ゅ' | 'ょ' | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ'
        | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ'
    ) {
        return None;
    }

    Some(match (first, second) {
        // K-row
        ('キ' | 'き', 'ャ' | 'ゃ') => "kya",
        ('キ' | 'き', 'ュ' | 'ゅ') => "kyu",
        ('キ' | 'き', 'ョ' | 'ょ') => "kyo",
        // S-row
        ('シ' | 'し', 'ャ' | 'ゃ') => "sha",
        ('シ' | 'し', 'ュ' | 'ゅ') => "shu",
        ('シ' | 'し', 'ョ' | 'ょ') => "sho",
        // T-row
        ('チ' | 'ち', 'ャ' | 'ゃ') => "cha",
        ('チ' | 'ち', 'ュ' | 'ゅ') => "chu",
        ('チ' | 'ち', 'ョ' | 'ょ') => "cho",
        // N-row
        ('ニ' | 'に', 'ャ' | 'ゃ') => "nya",
        ('ニ' | 'に', 'ュ' | 'ゅ') => "nyu",
        ('ニ' | 'に', 'ョ' | 'ょ') => "nyo",
        // H-row
        ('ヒ' | 'ひ', 'ャ' | 'ゃ') => "hya",
        ('ヒ' | 'ひ', 'ュ' | 'ゅ') => "hyu",
        ('ヒ' | 'ひ', 'ョ' | 'ょ') => "hyo",
        // M-row
        ('ミ' | 'み', 'ャ' | 'ゃ') => "mya",
        ('ミ' | 'み', 'ュ' | 'ゅ') => "myu",
        ('ミ' | 'み', 'ョ' | 'ょ') => "myo",
        // R-row
        ('リ' | 'り', 'ャ' | 'ゃ') => "rya",
        ('リ' | 'り', 'ュ' | 'ゅ') => "ryu",
        ('リ' | 'り', 'ョ' | 'ょ') => "ryo",
        // G-row
        ('ギ' | 'ぎ', 'ャ' | 'ゃ') => "gya",
        ('ギ' | 'ぎ', 'ュ' | 'ゅ') => "gyu",
        ('ギ' | 'ぎ', 'ョ' | 'ょ') => "gyo",
        // J-row
        ('ジ' | 'じ', 'ャ' | 'ゃ') => "ja",
        ('ジ' | 'じ', 'ュ' | 'ゅ') => "ju",
        ('ジ' | 'じ', 'ョ' | 'ょ') => "jo",
        // B-row
        ('ビ' | 'び', 'ャ' | 'ゃ') => "bya",
        ('ビ' | 'び', 'ュ' | 'ゅ') => "byu",
        ('ビ' | 'び', 'ョ' | 'ょ') => "byo",
        // P-row
        ('ピ' | 'ぴ', 'ャ' | 'ゃ') => "pya",
        ('ピ' | 'ぴ', 'ュ' | 'ゅ') => "pyu",
        ('ピ' | 'ぴ', 'ョ' | 'ょ') => "pyo",
        // Loanword digraphs
        ('テ' | 'て', 'ィ' | 'ぃ') => "ti",
        ('デ' | 'で', 'ィ' | 'ぃ') => "di",
        ('フ' | 'ふ', 'ァ' | 'ぁ') => "fa",
        ('フ' | 'ふ', 'ィ' | 'ぃ') => "fi",
        ('フ' | 'ふ', 'ェ' | 'ぇ') => "fe",
        ('フ' | 'ふ', 'ォ' | 'ぉ') => "fo",
        ('ウ' | 'う', 'ィ' | 'ぃ') => "wi",
        ('ウ' | 'う', 'ェ' | 'ぇ') => "we",
        ('ヴ', 'ァ' | 'ぁ') => "va",
        ('ヴ', 'ィ' | 'ぃ') => "vi",
        ('ヴ', 'ェ' | 'ぇ') => "ve",
        ('ヴ', 'ォ' | 'ぉ') => "vo",
        _ => return None,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kata_to_romaji_basic() {
        assert_eq!(kata_to_romaji("サイショ"), "saisho");
        assert_eq!(kata_to_romaji("ナカマ"), "nakama");
        assert_eq!(kata_to_romaji("セナカ"), "senaka");
    }

    #[test]
    fn test_kata_to_romaji_digraphs() {
        assert_eq!(kata_to_romaji("シャ"), "sha");
        assert_eq!(kata_to_romaji("チョ"), "cho");
        assert_eq!(kata_to_romaji("キュ"), "kyu");
        assert_eq!(kata_to_romaji("ジャ"), "ja");
    }

    #[test]
    fn test_kata_to_romaji_gemination() {
        assert_eq!(kata_to_romaji("ナッテ"), "natte");
        assert_eq!(kata_to_romaji("マッテ"), "matte");
        assert_eq!(kata_to_romaji("ニッポン"), "nippon");
    }

    #[test]
    fn test_kata_to_romaji_long_vowel() {
        assert_eq!(kata_to_romaji("ラーメン"), "raamen");
        assert_eq!(kata_to_romaji("コーヒー"), "koohii");
    }

    #[test]
    fn test_kata_to_romaji_loanwords() {
        assert_eq!(kata_to_romaji("ファイル"), "fairu");
        assert_eq!(kata_to_romaji("ティー"), "tii");
    }

    // ── Furigana bracketing (pure function — no MeCab binary needed) ────

    const MECAB_SHIRANAI_TENJOU: &str = "\
知ら\t動詞,自立,*,*,五段・ラ行,未然形,知る,シラ,シラ
ない\t助動詞,*,*,*,特殊・ナイ,基本形,ない,ナイ,ナイ
天井\t名詞,一般,*,*,*,*,天井,テンジョウ,テンジョー
だ\t助動詞,*,*,*,特殊・ダ,基本形,だ,ダ,ダ
EOS
";

    #[test]
    fn test_furigana_brackets_kanji_only() {
        let (furigana, _) = parse_mecab_output(MECAB_SHIRANAI_TENJOU).unwrap();
        assert!(furigana.contains("知ら[しら]"), "got: {furigana}");
        assert!(furigana.contains("天井[てんじょう]"), "got: {furigana}");
        assert!(!furigana.contains("ない["), "kana should not be bracketed: {furigana}");
        assert!(!furigana.contains("だ["), "kana should not be bracketed: {furigana}");
    }

    #[test]
    fn test_furigana_full_sentence() {
        let (furigana, _) = parse_mecab_output(MECAB_SHIRANAI_TENJOU).unwrap();
        assert_eq!(furigana, "知ら[しら]ない天井[てんじょう]だ");
    }

    #[test]
    fn test_romaji_from_mecab_morphemes() {
        let (_, romaji) = parse_mecab_output(MECAB_SHIRANAI_TENJOU).unwrap();
        assert!(romaji.contains("shira"), "got: {romaji}");
        assert!(romaji.contains("nai"), "got: {romaji}");
        assert!(romaji.contains("tenjou"), "got: {romaji}");
        assert!(romaji.contains("da"), "got: {romaji}");
    }

    const MECAB_MANGA: &str = "\
オレ\t名詞,代名詞,一般,*,*,*,オレ,オレ,オレ
が\t助詞,格助詞,一般,*,*,*,が,ガ,ガ
最初\t名詞,副詞可能,*,*,*,*,最初,サイショ,サイショ
に\t助詞,格助詞,一般,*,*,*,に,ニ,ニ
仲間\t名詞,一般,*,*,*,*,仲間,ナカマ,ナカマ
に\t助詞,格助詞,一般,*,*,*,に,ニ,ニ
なっ\t動詞,自立,*,*,五段・ラ行,連用タ接続,なる,ナッ,ナッ
て\t助詞,接続助詞,*,*,*,*,て,テ,テ
背中\t名詞,一般,*,*,*,*,背中,セナカ,セナカ
守っ\t動詞,自立,*,*,五段・ラ行,連用タ接続,守る,マモッ,マモッ
て\t助詞,接続助詞,*,*,*,*,て,テ,テ
やる\t動詞,非自立,*,*,五段・ラ行,基本形,やる,ヤル,ヤル
よ\t助詞,終助詞,*,*,*,*,よ,ヨ,ヨ
!\t記号,一般,*,*,*,*,!,!,!
EOS
";

    #[test]
    fn test_furigana_manga_sentence() {
        let (furigana, _) = parse_mecab_output(MECAB_MANGA).unwrap();
        assert!(furigana.contains("最初[さいしょ]"), "got: {furigana}");
        assert!(furigana.contains("仲間[なかま]"), "got: {furigana}");
        assert!(furigana.contains("背中[せなか]"), "got: {furigana}");
        assert!(furigana.contains("守っ[まもっ]"), "got: {furigana}");
        assert!(!furigana.contains("オレ["), "katakana should not be bracketed: {furigana}");
        assert!(!furigana.contains("やる["), "hiragana should not be bracketed: {furigana}");
    }

    #[test]
    fn test_romaji_manga_sentence() {
        let (_, romaji) = parse_mecab_output(MECAB_MANGA).unwrap();
        assert!(romaji.contains("saisho"), "got: {romaji}");
        assert!(romaji.contains("nakama"), "got: {romaji}");
        assert!(romaji.contains("senaka"), "got: {romaji}");
    }

    #[test]
    fn test_parse_empty_mecab_output() {
        assert!(parse_mecab_output("EOS\n").is_none());
        assert!(parse_mecab_output("").is_none());
    }

    #[test]
    fn test_parse_no_reading_field() {
        let output = "hello\t記号,一般,*,*,*,*\nEOS\n";
        let (furigana, _) = parse_mecab_output(output).unwrap();
        assert_eq!(furigana, "hello");
    }

    #[test]
    fn test_has_kanji() {
        assert!(has_kanji("天井"));
        assert!(has_kanji("知ら"));
        assert!(!has_kanji("おはよう"));
        assert!(!has_kanji("オレ"));
        assert!(!has_kanji("hello"));
        assert!(!has_kanji("!"));
    }

    #[test]
    fn test_kata_to_hira() {
        assert_eq!(kata_to_hira("テンジョウ"), "てんじょう");
        assert_eq!(kata_to_hira("サイショ"), "さいしょ");
        assert_eq!(kata_to_hira("さいしょ"), "さいしょ");
        assert_eq!(kata_to_hira("ABC"), "ABC");
    }

    // ── Integration (requires MeCab + dictionary) ───────────────────────

    #[test]
    fn test_annotate_populates_fields() {
        if let Some(result) = annotate("食べる") {
            assert!(!result.furigana.is_empty(), "furigana should be set");
            assert!(!result.romaji.is_empty(), "romaji should be set");
        }
    }

    #[test]
    fn test_annotate_furigana_only_skips_romaji() {
        if let Some(result) = annotate_furigana_only("食べる") {
            assert!(!result.furigana.is_empty(), "furigana should be set");
            assert!(result.romaji.is_empty(), "romaji should be empty");
        }
    }

    #[test]
    fn test_annotate_empty_text() {
        assert!(annotate("").is_none());
        assert!(annotate("   ").is_none());
    }
}

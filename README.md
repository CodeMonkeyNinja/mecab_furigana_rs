# mecab-furigana-rs

MeCab-based furigana and romaji annotation for Japanese text.  No Python, no kakasi.

Converts Japanese text containing kanji into bracketed furigana format (`天井[てんじょう]`)
and Hepburn romaji (`tenjou`).  Uses MeCab's morphological analyzer (CLI) for
dictionary-accurate readings at ~5 ms per invocation.

## Usage

```rust
let result = mecab_furigana_rs::annotate("知らない天井だ").unwrap();
assert_eq!(result.furigana, "知ら[しら]ない天井[てんじょう]だ");
assert!(result.romaji.contains("tenjou"));
```

### Word segmentation

MeCab segments ambiguous kana strings into individual morphemes.  The classic
example `すもももももももものうち` ("plums and peaches are both types of peach")
becomes:

```rust
let result = mecab_furigana_rs::annotate("すもももももももものうち").unwrap();
let words: Vec<&str> = result.morphemes.iter().map(|m| m.surface.as_str()).collect();
assert_eq!(words, &["すもも", "も", "もも", "も", "もも", "の", "うち"]);
```

### Morpheme data

Each `Morpheme` includes surface, reading, romaji, dictionary base form, and POS:

```rust
let result = mecab_furigana_rs::annotate("知らない天井だ").unwrap();
let m = &result.morphemes[0];
assert_eq!(m.surface,   "知ら");   // as it appears in text
assert_eq!(m.reading,   "しら");   // hiragana reading
assert_eq!(m.romaji,    "shira");  // Hepburn romaji
assert_eq!(m.base_form, "知る");   // dictionary/lemma form
assert_eq!(m.pos,       "動詞");   // part of speech
```

### Furigana only (skip romaji)

```rust
let result = mecab_furigana_rs::annotate_furigana_only("食べる").unwrap();
// result.furigana = "食[た]べる"
// result.romaji = "" (empty)
// result.morphemes is still populated
```

### Pure helpers (no MeCab required)

```rust
// Katakana to romaji (Hepburn)
assert_eq!(mecab_furigana_rs::kata_to_romaji("ラーメン"), "raamen");

// Katakana to hiragana
assert_eq!(mecab_furigana_rs::kata_to_hira("テンジョウ"), "てんじょう");

// Kanji detection
assert!(mecab_furigana_rs::has_kanji("天井"));

// Parse raw MeCab output (for testing or custom pipelines)
let (furigana, romaji, morphemes) = mecab_furigana_rs::parse_mecab_output(mecab_stdout).unwrap();
```

## Prerequisites

Install MeCab and a UTF-8 dictionary:

```bash
# Debian / Ubuntu
sudo apt install mecab mecab-naist-jdic

# Arch
pacman -S mecab mecab-naist-jdic
```

The crate auto-discovers dictionaries from standard system paths
(`/usr/share/mecab/dic/`, `/usr/lib/mecab/dic/`, `/var/lib/mecab/dic/`),
preferring `naist-jdic` (richer: names, neologisms) over `ipadic-utf8`.

## How it works

1. Spawns `mecab -r /dev/null -d <dict>` and pipes text to stdin
2. Parses MeCab's tab-separated morpheme output (surface + katakana reading)
3. Kanji surfaces get bracketed hiragana: `天井` + `テンジョウ` = `天井[てんじょう]`
4. Romaji is derived from katakana readings via Hepburn romanization tables
   - Digraphs: キャ=kya, シュ=shu, チョ=cho (50+ combinations)
   - Gemination: ッ doubles next consonant (ナッテ=natte)
   - Long vowels: ー repeats previous vowel (ラーメン=raamen)
   - Loanwords: ファ=fa, ティ=ti, ヴァ=va

## Platform support

Full functionality on Linux.  On other platforms, `annotate()` returns `None`;
the pure conversion helpers (`kata_to_romaji`, `kata_to_hira`, `has_kanji`,
`parse_mecab_output`) work everywhere.

## Background

Due to past difficulties integrating [Kakasi](https://github.com/HidekiAI/kakasi)
as a library, this crate takes the simpler approach of shelling out to the MeCab
CLI (which users pre-install alongside dictionaries).

## License

- This crate: MIT
- MeCab: BSD

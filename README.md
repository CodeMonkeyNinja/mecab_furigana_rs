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

### Output formats

The crate offers two rendering targets from the same annotated data:

**1. Bracket text** (TUI / TTY / CLI) — `annotate()` embeds readings inline:

```
知ら[しら]ない天井[てんじょう]だ
```

Suitable for terminal output, log files, plain-text display, or any
environment without HTML/CSS.

**2. HTML span markup** (Web) — `furigana_to_html()` produces accessible
CSS-grid ruby display suitable for browsers and rich UIs:

```rust
let html = mecab_furigana_rs::furigana_to_html("知ら[しら]ない天井[てんじょう]だ");
// → <span class="furigana"><span class="read">しら</span><span class="base">知ら</span></span>ない<span class="furigana"><span class="read">てんじょう</span><span class="base">天井</span></span>だ
```

The downstream CSS contract expects:

```css
.furigana { display: inline-grid; place-items: center; grid-template-rows: auto auto; }
.furigana .read { font-size: 0.55em; line-height: 1; grid-row: 1; white-space: nowrap; }
.furigana .base { grid-row: 2; }
```

See [`furigana-demo.html`](./furigana-demo.html) in the repo root for a
live preview you can open in any browser.

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

// Bracket-string → HTML furigana renderer (no MeCab needed)
let html = mecab_furigana_rs::furigana_to_html("知ら[しら]ない天井[てんじょう]だ");

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

# macOS / other: install mecab + a UTF-8 dictionary however you prefer,
# then point the crate at your dictionary:
export MECAB_DICT_DIR=/path/to/your/mecab/dic
```

The crate auto-discovers dictionaries by probing the cross product of:

- **Roots**: `/usr/share/mecab/dic`, `/var/lib/mecab/dic`, `/usr/lib/mecab/dic`,
  `/usr/local/lib/mecab/dic`, `/opt/homebrew/lib/mecab/dic` (macOS Homebrew),
  `/opt/local/lib/mecab/dic` (MacPorts)
- **Names** (preference order): `mecab-ipadic-neologd`, `ipadic-neologd`,
  `naist-jdic`, `ipadic-utf8`, `ipadic`, `mecab-unidic-neologd`,
  `unidic-neologd`, `unidic-cwj`, `unidic`, `jumandic-utf8`, `juman-utf8`,
  `jumandic`

Names-first iteration means a NEologd install anywhere beats a `naist-jdic`
elsewhere.  Each candidate is sanity-checked against its `dicrc` and skipped
if it declares EUC-JP / Shift_JIS / CP932 (so the bare `ipadic` directory,
which is UTF-8 on macOS Homebrew but EUC-JP on Debian, only matches where
it's actually UTF-8).

For any path not on that list, set `MECAB_DICT_DIR` directly.

> **POS schema caveat**: IPAdic / NAIST jdic / NEologd share the IPA POS
> tagset, which is what this crate's `Morpheme.pos` values reflect.  UniDic
> and JUMAN dicts use different POS category strings (top-level categories
> like `名詞`/`動詞` mostly still appear, but sub-detail differs) — they
> work, but downstream code that pattern-matches on `pos_detail` may need
> adjusting.

### User dictionaries (rare kanji, names, neologisms)

When MeCab can't find a word, you can layer a user dictionary on top of
the system dict by setting `MECAB_USER_DICT` to a compiled `.dic` file
(or a comma-separated list, matching `mecab -u a.dic,b.dic`):

```bash
export MECAB_USER_DICT=./mydict.dic
export MECAB_USER_DICT=./names.dic,./neologisms.dic
```

Build the `.dic` once with the `mecab-dict-index` tool that ships with MeCab:

```bash
# CSV row: surface,left-id,right-id,cost,POS,POS1,POS2,POS3,conj-form,conj-type,base,reading-kata,pron-kata
# (use 0,0 for left/right IDs and a small cost — MeCab will fill them in)
echo '兎角,0,0,5000,名詞,固有名詞,*,*,*,*,兎角,トカク,トカク' > mydict.csv

mecab-dict-index -d "$MECAB_DICT_DIR" -u mydict.dic \
                 -f UTF-8 -t UTF-8 mydict.csv
```

`mecab-dict-index` is a sibling binary to `mecab` itself, so it's available
on every platform where MeCab is installed (Debian: `/usr/lib/mecab/...`;
Homebrew: under `libexec/mecab/`; Windows: in the MeCab install `bin/`).

### `.env` support

Both `MECAB_DICT_DIR` and `MECAB_USER_DICT` may also be set in a `.env`
file in the process working directory.  Real environment variables always
take precedence; `.env` only fills in unset ones.  See `.env.sample` for
the expected format.  `.env` itself is gitignored.

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

Works on any platform where `mecab` is on `$PATH` and a dictionary is
discoverable (standard Linux paths or `MECAB_DICT_DIR`).  The pure conversion
helpers (`kata_to_romaji`, `kata_to_hira`, `has_kanji`, `parse_mecab_output`)
work everywhere regardless of MeCab availability.

## Background

Due to past difficulties integrating [Kakasi](https://github.com/HidekiAI/kakasi)
as a library, this crate takes the simpler approach of shelling out to the MeCab
CLI (which users pre-install alongside dictionaries).

## License

- This crate: MIT
- MeCab: BSD

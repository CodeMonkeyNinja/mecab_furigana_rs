# Furigana Segment Parser + HTML Span Renderer

## Motivation

The crate currently emits a flat bracket string (e.g. `知ら[しら]ない天井[てんじょう]だ`). Downstream consumers like [lenzu](https://github.com/CodeMonkeyNinja/lenzu) need to render furigana as small hiragana **above** each kanji in an HTML overlay. This requires:

1. Parsing the bracket format back into structured segments
2. Rendering segments to HTML span markup

Both are general furigana operations that belong in this crate rather than being reimplemented in every downstream project.

## Proposed API

### New types

```rust
/// A single annotated segment of Japanese text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// A kanji character or sequence with its reading.
    Kanji {
        text: String,
        reading: Option<String>,
    },
    /// Non-kanji text (kana, punctuation, ASCII, whitespace).
    Other(String),
}
```

### New functions

```rust
/// Parse a bracketed furigana string into structured segments.
///
/// "知ら[しら]ない天井[てんじょう]だ"
/// → [
///     Kanji { text: "知ら", reading: Some("しら") },
///     Other("ない"),
///     Kanji { text: "天井", reading: Some("てんじょう") },
///     Other("だ"),
///   ]
pub fn parse_furigana(input: &str) -> Vec<Segment>

/// Render segments as HTML span markup for CSS-based ruby display.
///
/// The output uses `<span class="furigana">` as a CSS grid container
/// with two children:
/// - `<span class="read">` — the reading (small text above)
/// - `<span class="base">` — the kanji (baseline text)
///
/// Non-kanji segments pass through as-is (HTML-escaped).
pub fn segments_to_html(segments: &[Segment]) -> String

/// Convenience wrapper: parse + render in one call.
///
/// Equivalent to `segments_to_html(&parse_furigana(input))`.
pub fn furigana_to_html(input: &str) -> String
```

## Parser: `parse_furigana`

### Input format

The bracket format produced by this crate's `annotate()`:

```
<surface>[<reading>]   for kanji-containing morphemes
<surface>              for everything else
```

Concatenated in MeCab output order.

### Parsing rules

Scan left to right:

1. If the next character is CJK (`\p{Han}`: U+4E00–U+9FFF, U+3400–U+4DBF), consume a run of consecutive CJK characters as the `text`. If followed immediately by `[`, consume the bracketed content as `reading` (may be empty). Otherwise, `reading` is `None`.

2. Non-CJK characters pass through as `Other(String)` segments, coalesced for efficiency (consecutive non-CJK runs can be merged into a single `Other` segment).

3. A `[` without preceding CJK characters is treated as literal text (part of `Other`).

4. Empty reading (`漢字[]`) yields `Kanji { text: "漢字", reading: Some("") }`.

### Examples

| Input | Segments | Notes |
|---|---|---|
| `食[た]べる` | `[Kanji("食", Some("た")), Other("べる")]` | Basic |
| `食[た]べ物[もの]` | `[Kanji("食", Some("た")), Other("べ"), Kanji("物", Some("もの"))]` | Multiple |
| `日本語` | `[Other("日本語")]` | No brackets |
| `` | `[]` | Empty |
| `[hello]` | `[Other("[hello]")]` | Literal bracket |
| `漢字[]` | `[Kanji("漢字", Some(""))]` | Empty reading |

## Renderer: `segments_to_html`

### HTML output format

```html
<span class="furigana"><span class="read">た</span><span class="base">食</span></span>べ
```

- Kanji with reading: wrapped in `<span class="furigana">` containing `.read` and `.base` spans.
- Kanji without reading: rendered as plain text (same as `Other`).
- `Other` segments: HTML-escaped via `html_escape()` (escape `<`, `>`, `&`, `"`).

### CSS contract

The downstream CSS expects (in lenzu's `styles.css`):

```css
.furigana {
  display: inline-grid;
  place-items: center;
  grid-template-rows: auto auto;
}
.furigana .read {
  font-size: 0.55em;
  line-height: 1;
  grid-row: 1;
  white-space: nowrap;
}
.furigana .base {
  grid-row: 2;
}
```

This class naming is part of the public contract and must remain stable.

### Examples (input → HTML)

| Input | Output |
|---|---|
| `食[た]べ物[もの]` | `<span class="furigana"><span class="read">た</span><span class="base">食</span></span>べ<span class="furigana"><span class="read">もの</span><span class="base">物</span></span>` |
| `日本語` | `日本語` |
| `漢字[]` | `<span class="furigana"><span class="read"></span><span class="base">漢字</span></span>` |
| `abc[]123` | `abc[]123` (bracket after non-kanji → literal) |

## No-dependency constraint

The crate has **zero external dependencies** (only `std`). The HTML renderer must implement its own trivial HTML escaping rather than pulling in a crate.

## Tests

### Unit tests for `parse_furigana`

| Test name | Input | Expected |
|---|---|---|
| `parse_basic` | `"食[た]べる"` | `[Kanji("食", Some("た")), Other("べる")]` |
| `parse_multiple` | `"食[た]べ物[もの]"` | `[Kanji("食", Some("た")), Other("べ"), Kanji("物", Some("もの"))]` |
| `parse_no_brackets` | `"日本語"` | `[Other("日本語")]` |
| `parse_empty` | `""` | `[]` |
| `parse_bracket_without_kanji` | `"[hello]"` | `[Other("[hello]")]` |
| `parse_empty_reading` | `"漢字[]"` | `[Kanji("漢字", Some(""))]` |
| `parse_consecutive` | `"東京[とうきょう]大阪[おおさか]"` | `[Kanji("東京", Some("とうきょう")), Kanji("大阪", Some("おおさか"))]` |
| `parse_kanji_without_reading` | `"東京"` | `[Other("東京")]` (kanji without brackets = no reading) |
| `parse_mixed_cjk_non_cjk` | `"abc123!@#"` | `[Other("abc123!@#")]` |

### Unit tests for `segments_to_html`

| Test name | Input segments | Expected HTML |
|---|---|---|
| `render_basic` | `[Kanji("食", Some("た")), Other("べる")]` | `<span class="furigana"><span class="read">た</span><span class="base">食</span></span>べる` |
| `render_empty_segments` | `[]` | `""` |
| `render_kanji_no_reading` | `[Other("東京")]` | `"東京"` |
| `render_empty_reading` | `[Kanji("漢字", Some(""))]` | `<span class="furigana"><span class="read"></span><span class="base">漢字</span></span>` |
| `render_html_escaping` | `[Other("<script>")]` | `&lt;script&gt;` |

### Smoke test for `furigana_to_html`

| Test name | Input | Expected |
|---|---|---|
| `smoke_roundtrip` | `"食[た]べ物[もの]"` | Full HTML span output (matches `render_basic` + `render` for `食べ物`) |

## Implementation notes

- `parse_furigana` should use `char_indices()` or `chars()` with manual iteration. No regex dependency needed — the grammar is simple enough for a state machine.
- `segments_to_html` should pre-allocate with `String::with_capacity(input.len() * 2)` for reasonable growth.
- HTML escaping: replace `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&quot;`. Only needed in `Other` segments (reading and kanji text are trusted CJK strings, but safe-escaping them costs nothing).

## Versioning

These additions are **backward compatible** — no existing API changes. The new types and functions simply extend the public surface. Bump minor version.

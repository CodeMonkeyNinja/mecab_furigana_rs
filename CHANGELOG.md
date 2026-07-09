# Changelog

## [1.0.0] — 2026-07-09

- **Stable API milestone** — no breaking changes planned. Furigana
  annotation, romaji conversion, HTML `<ruby>` rendering, and morpheme
  parsing are mature and production-tested within the Lenzu ecosystem.

## [0.4.0] — 2026-05-xx

- **HTML `<ruby>` output** — `furigana_to_html()` produces W3C-native
  `<ruby><rt>` annotations (replacing CSS-grid spans from v0.3.0).
- Furigana segment parser for bracket-string → HTML rendering.

## [0.3.0] — 2026-05-xx

- **User dictionaries** — `MECAB_USER_DICT` env var for rare kanji,
  names, and neologisms (comma-separated `.dic` files).
- **`.env` support** — `MECAB_DICT_DIR` and `MECAB_USER_DICT` may be
  set in a `.env` file in the working directory.
- **Widened dict probe list** — 17+ dictionary paths across Linux
  distros and macOS Homebrew/MacPorts.
- **Missing-reading fallback** — emit `surface[]` for kanji that MeCab
  can't find a reading for.
- CI: auto-publish on tag push; `workflow_dispatch` trigger.
- Require_mecab() validation for binary + dictionary at startup.

## [0.2.0] — 2026-05-xx

- **`MECAB_DICT_DIR` env var** — remove Linux-only gate; crate works on
  macOS and other platforms.
- GitHub Actions CI workflow.
- Docs: macOS install hints.

## [0.1.0] — 2026-05-xx

- Initial release: MeCab-based furigana and romaji annotation.
- `annotate()` — bracketed furigana format (`天井[てんじょう]`).
- `annotate_furigana_only()` — skip romaji computation.
- Pure helpers: `kata_to_romaji()`, `kata_to_hira()`, `has_kanji()`.
- `Morpheme` struct with surface, reading, romaji, base form, POS.
- Word segmentation via MeCab morphemes.
- Hepburn romanization with digraph, gemination, long vowel, and
  loanword support.
- Zero external dependencies (std only).

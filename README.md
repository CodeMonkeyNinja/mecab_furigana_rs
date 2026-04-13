# mecab_furigana_rs
A very trivial method integrating MeCab to transform string with kanji to have furigana in similar format as Kakasi


This tool converts Japanese text to a Kakasi-like format by identifying kanji and their readings. It uses MeCab to perform morphological analysis, extracting the surface form and phonetic reading (yomi) for each token. For tokens containing kanji, the algorithm replaces the kanji substring with the format `kanji[hiragana]`, where the hiragana reading is derived from MeCab's output. The transformation preserves the original string's structure, ensuring accurate positional alignment between kanji and their furigana, and outputs the result in the desired `知[し]らない` style.

Note that due to past problems I've hade with [Kakasi](https://github.com/HidekiAI/kakasi) to integrate it as `lib`, for MeCab, I will assume to have the users preinstall the CLI version (alongside with the dicts) as prerequisite for their target platform;

On the average, the whole furigana'ization takes about 5ms on Debian.

## LICENSE
- Kakasi is licensed under GPLv2
- MeCab is licensed under BSD

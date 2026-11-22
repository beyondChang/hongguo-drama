//! The domain logic the hongguo modules share.
//!
//! Several modules read the same three things: the season a title names, the source a series id
//! belongs to, and how two titles compare. `watch-history.js` holds them, and
//! `series-search.js`, `nas-export.js` and `history-service.js` all reach back into it for the
//! answers. The ports keep that one copy here, so a series the search finds and a series the
//! history shows are read the same way.

use hongguo_js::{js_string, string_or_empty};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

/// Decoder signatures and merge planning, used by the download pipeline in all three shells.
pub mod merge_plan;

/// Release-tag version comparison, used by the update check.
pub mod app_update;

/// Pure danmaku value normalization and page parsing.
pub mod danmaku;

hongguo_js::js_regex!(digit_regex, r"^[0-9]{6,30}$");
hongguo_js::js_regex!(xifan_regex, r"^xifan:[a-zA-Z0-9_-]+:[a-zA-Z0-9_-]+$");
hongguo_js::js_regex!(hema_regex, r"^hema:[0-9]+$");
hongguo_js::js_regex!(
    season_regex,
    r"(?i)^(.*?)(?:(?:第)?([一二三四五六七八九十百零两0-9]+)[季部]|[（(](?:第)?([一二三四五六七八九十百零两0-9]+)(?:[季部])?[）)]|season([0-9]+))$"
);
hongguo_js::js_regex!(
    whitespace_regex,
    r"[\t\n\v\f\r \u{00a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]+"
);
hongguo_js::js_regex!(punctuation_regex, r"[\p{P}\p{S}]");
hongguo_js::js_regex!(season_number_regex, r"[零〇一二三四五六七八九十百两0-9]+");
hongguo_js::js_regex!(ten_hundred_regex, r"[十百]");

/// What a title says about its season.
#[derive(Clone, Debug, PartialEq)]
pub struct SeasonTitle {
    /// The title without its season marker.
    pub base: String,
    /// The season number the title names, `1` when it names none.
    pub season: f64,
    /// Whether the title named a season at all.
    pub explicit: bool,
    /// Whether the title named a season that cannot be read, such as a year.
    pub invalid: bool,
}

/// The source a series id belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Hongguo,
    Xifan,
    Hema,
}

impl Source {
    /// The name the modules use for this source.
    pub fn name(self) -> &'static str {
        match self {
            Self::Hongguo => "hongguo",
            Self::Xifan => "xifan",
            Self::Hema => "hema",
        }
    }
}

/// `sourceOf(id)`: the source a series id belongs to, `None` when it names no source.
pub fn source_of(id: &str) -> Option<Source> {
    if digit_regex().is_match(id) {
        Some(Source::Hongguo)
    } else if xifan_regex().is_match(id) {
        Some(Source::Xifan)
    } else if hema_regex().is_match(id) {
        Some(Source::Hema)
    } else {
        None
    }
}

/// `sourceOf(id)` for a value that came from JS: the id is read as text first.
pub fn source_of_value(id: &Value) -> Option<Source> {
    source_of(&js_string(id))
}

/// `seasonTitle(title)`: the season a title names.
pub fn season_title(title: &Value) -> SeasonTitle {
    let raw = string_or_empty(title);
    let normalized: String = raw.nfkc().collect();
    let text = whitespace_regex()
        .replace_all(&normalized, "")
        .to_lowercase();

    let Some(captures) = season_regex().captures(&text) else {
        return SeasonTitle {
            base: text,
            season: 1.0,
            explicit: false,
            invalid: false,
        };
    };
    let group = |index: usize| captures.get(index).map(|m| m.as_str().to_string());
    let Some(value) = group(2).or_else(|| group(3)).or_else(|| group(4)) else {
        return SeasonTitle {
            base: text,
            season: 1.0,
            explicit: false,
            invalid: false,
        };
    };

    let mut season = hongguo_js::string_to_number(&value);
    if !season.is_finite() {
        let mut n = 0.0;
        let mut digit = 0.0;
        for ch in value.chars() {
            if ch == '十' || ch == '百' {
                let unit = if digit == 0.0 { 1.0 } else { digit };
                n += unit * if ch == '十' { 10.0 } else { 100.0 };
                digit = 0.0;
            } else {
                // `digit = digits[char] || 0`: an unknown character counts as zero.
                digit = chinese_digit(ch).map_or(0.0, |d| d as f64);
            }
        }
        season = if ten_hundred_regex().is_match(&value) {
            n + digit
        } else {
            // `Number([...value].map(char => digits[char]).join(''))`: an unknown character
            // becomes `undefined`, which `join` writes as an empty string.
            let joined: String = value
                .chars()
                .filter_map(|ch| chinese_digit(ch).and_then(|d| char::from_digit(d, 10)))
                .collect();
            hongguo_js::string_to_number(&joined)
        };
    }

    // A bare bracket number can be a year or other metadata: only modest seasons qualify.
    if group(3).is_some() && season > 99.0 {
        return SeasonTitle {
            base: text,
            season: 1.0,
            explicit: false,
            invalid: false,
        };
    }
    if !hongguo_js::number_is_safe_integer(season) || season < 1.0 {
        return SeasonTitle {
            base: text,
            season: 1.0,
            explicit: false,
            invalid: true,
        };
    }
    SeasonTitle {
        base: captures
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or(text),
        season,
        explicit: true,
        invalid: false,
    }
}

/// The Chinese digit values the season parser knows.
fn chinese_digit(ch: char) -> Option<u32> {
    match ch {
        '零' => Some(0),
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

/// `normalize(title)`: drop punctuation and symbols, then mask season numbers.
pub fn normalize_title(title: &str) -> String {
    let stripped = punctuation_regex().replace_all(title, "");
    season_number_regex()
        .replace_all(&stripped, "#")
        .to_string()
}

/// `similarTitles(left, right)`: at most two edits and a fifth of the title.
pub fn similar_titles(left: &str, right: &str) -> bool {
    let a: Vec<char> = left.chars().collect();
    let b: Vec<char> = right.chars().collect();
    let length = a.len().max(b.len());
    let limit = hongguo_js::js_min(2.0, (length as f64 / 5.0).floor());
    if hongguo_js::js_min(a.len() as f64, b.len() as f64) < 4.0
        || length as f64 > 200.0
        || (a.len() as f64 - b.len() as f64).abs() > limit
    {
        return false;
    }
    let mut previous: Vec<f64> = (0..=b.len()).map(|index| index as f64).collect();
    for i in 1..=a.len() {
        let mut current = vec![0.0; b.len() + 1];
        current[0] = i as f64;
        for j in 1..=b.len() {
            let substitution = if a[i - 1] == b[j - 1] { 0.0 } else { 1.0 };
            current[j] = hongguo_js::js_min_all([
                current[j - 1] + 1.0,
                previous[j] + 1.0,
                previous[j - 1] + substitution,
            ]);
        }
        if hongguo_js::js_min_all(current.iter().copied()) > limit {
            return false;
        }
        previous = current;
    }
    previous[b.len()] <= limit
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_season_a_title_names() {
        let named = |title: &str| season_title(&json!(title));
        assert_eq!(named("春日来信第二季").season, 2.0);
        assert_eq!(named("春日来信第二季").base, "春日来信");
        assert!(named("春日来信第二季").explicit);
        assert_eq!(named("春日来信").season, 1.0);
        assert!(!named("春日来信").explicit);
        assert_eq!(named("春日来信Season3").season, 3.0);
        assert_eq!(named("春日来信（10）").season, 10.0);
        assert_eq!(named("春日来信（第九季）").season, 9.0);
        assert_eq!(named("春日来信（第2季）").season, 2.0);
        // A bracket number can be a year: the title then names no season at all.
        let year = named("春日来信（2024）");
        assert_eq!(year.season, 1.0);
        assert!(!year.explicit);
        assert!(!year.invalid);
        // A season below one cannot be read.
        let zero = named("春日来信（0）");
        assert!(zero.invalid);
        assert_eq!(zero.season, 1.0);
    }

    #[test]
    fn reads_the_source_an_id_belongs_to() {
        assert_eq!(source_of("1000002"), Some(Source::Hongguo));
        assert_eq!(source_of("xifan:ab:cd"), Some(Source::Xifan));
        assert_eq!(source_of("hema:12"), Some(Source::Hema));
        assert_eq!(source_of("nope"), None);
    }
}

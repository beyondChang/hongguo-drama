//! The Rust port of `src/native/watch-history.js`.
//!
//! The JavaScript semantics live in the `hongguo-js` crate and the domain logic the modules
//! share lives in `hongguo-core`, so every port reads `undefined`, truthiness, season titles and
//! series sources the same way. After changing a signature, run `rust-anything refresh` to
//! regenerate the bindings. `rust-anything diff` checks behavior against what the source
//! recorded.

#![forbid(unsafe_code)]

mod pinned;

use hongguo_core::{normalize_title, similar_titles};
use hongguo_js::*;
use serde_json::{Map, Value, json};

/// `totalEpisodes(series)`: the larger of the declared total and the episode list length.
fn total_episodes(series: &Value) -> f64 {
    let declared = optional_chain(series, &["total"])
        .map(number_or_zero)
        .unwrap_or(0.0);
    let listed = optional_chain(series, &["episodes"])
        .and_then(js_length)
        .unwrap_or(0.0);
    js_max(declared, listed)
}

/// `isWatched(record)`: a record that has been played and carries a usable position.
fn is_watched(record: &Value) -> bool {
    if is_nullish(record) {
        return false;
    }
    let watched = prop(record, "watched") == &Value::Bool(true)
        || to_number(prop(record, "currentTime")) > 0.0;
    watched
        && number_is_finite(prop(record, "updatedAt"))
        && number_is_integer(prop(record, "vid_index"))
        && to_number(prop(record, "vid_index")) > 0.0
}

/// `familyKey(series)`: the internal form, without the public wrapper's `Result`.
fn family_key_value(series: &Value) -> String {
    let parsed = hongguo_core::season_title(
        &optional_chain(series, &["series_title"])
            .cloned()
            .unwrap_or(Value::Null),
    );
    let id = optional_chain(series, &["series_id"])
        .map(string_or_empty)
        .unwrap_or_default();
    let Some(source) = hongguo_core::source_of(&id) else {
        return String::new();
    };
    if parsed.base.is_empty() || parsed.invalid {
        return String::new();
    }
    format!("{}\u{0}{}", source.name(), parsed.base)
}

/// `seasonOf(series)`: the internal form, without the public wrapper's `Result`.
fn season_of_value(series: &Value) -> f64 {
    let parsed = hongguo_core::season_title(
        &optional_chain(series, &["series_title"])
            .cloned()
            .unwrap_or(Value::Null),
    );
    if parsed.invalid { 0.0 } else { parsed.season }
}

/// Port of `listHistory` (`src/native/watch-history.js:10`).
pub fn list_history(series: Value, playback: Value, show_sequels: Value) -> Result<Value, String> {
    let items = filter_array(&series, "series", "filter")?;
    // `showSequels = true`: the default fires only for `undefined`.
    let show_sequels = if is_undefined(&show_sequels) {
        true
    } else {
        truthy(&show_sequels)
    };

    // `series.filter(item => isWatched(playback[item.series_id]))`: the lookup happens for
    // every item, so a nullish `playback` throws before the filter can drop anything.
    let mut watched: Vec<Value> = Vec::new();
    let mut records: Vec<Value> = Vec::new();
    for item in &items {
        let key = index_key(prop(item, "series_id"));
        let record = js_index_get(&playback, &key)?;
        if is_watched(record) {
            watched.push(item.clone());
            records.push(record.clone());
        }
    }

    // `latestByFamily`: the highest season seen per family, in first-seen order.
    let mut families: Vec<(String, f64)> = Vec::new();
    for item in &watched {
        let key = family_key_value(item);
        if key.is_empty() {
            continue;
        }
        let season = season_of_value(item);
        match families.iter().position(|(k, _)| *k == key) {
            Some(index) => families[index].1 = js_max(families[index].1, season),
            None => families.push((key, season)),
        }
    }
    let latest_of = |key: &str| -> f64 {
        families
            .iter()
            .find(|(k, _)| k == key)
            .map_or(0.0, |(_, s)| *s)
    };

    let mut rows: Vec<Value> = Vec::with_capacity(watched.len());
    for (index, item) in watched.iter().enumerate() {
        let record = &records[index];
        let family = family_key_value(item);
        let season = season_of_value(item);
        let latest_season = latest_of(&family);

        let sequels = if show_sequels && is_array(prop(record, "sequels")) {
            find_sequels_value(item, prop(record, "sequels"), latest_season)
        } else {
            Vec::new()
        };

        // `confirmedIds` holds strings; the lookup compares them with the raw id.
        let confirmed_ids: Vec<Value> = sequels
            .iter()
            .map(|c| Value::String(js_string(prop(c, "series_id"))))
            .collect();

        let seen: Vec<Value> = array_or_empty(prop(record, "seenSequelIds"));
        let unread_sequel_ids: Vec<Value> = sequels
            .iter()
            .filter(|candidate| {
                let id = prop(candidate, "series_id");
                !seen.iter().any(|s| same_value_zero(s, id))
            })
            .map(|candidate| prop(candidate, "series_id").clone())
            .collect();

        let total = js_max(
            total_episodes(item),
            number_or_zero(prop(record, "knownTotal")),
        );

        let similar: Vec<Value> = if show_sequels {
            let mut candidates = array_or_empty(prop(record, "similarSeries"));
            candidates.extend(watched.iter().cloned());
            find_similar_series_value(item, &candidates)
                .into_iter()
                .filter(|candidate| {
                    let id = prop(candidate, "series_id");
                    !confirmed_ids.iter().any(|c| same_value_zero(c, id))
                })
                .collect()
        } else {
            Vec::new()
        };

        let completed = total > 0.0
            && number_is_integer(prop(record, "completedEpisode"))
            && to_number(prop(record, "completedEpisode")) >= total;
        let has_updates =
            to_number(prop(record, "unreadEpisodes")) > 0.0 || !unread_sequel_ids.is_empty();
        let is_latest_season = !family.is_empty() && season == latest_season;
        let has_higher_season = !family.is_empty() && season < latest_season;

        rows.push(json!({
            "series_id": prop(item, "series_id"),
            "series_title": prop(item, "series_title"),
            "cover": prop_or(item, "cover", Value::String(String::new())),
            "total": number(total),
            "completed": completed,
            "vid_index": prop(record, "vid_index"),
            "currentTime": prop(record, "currentTime"),
            "updatedAt": prop(record, "updatedAt"),
            "unreadEpisodes": prop_or(record, "unreadEpisodes", number(0.0)),
            "sequels": sequels,
            "unreadSequelIds": unread_sequel_ids,
            "similarSeries": similar,
            "hasUpdates": has_updates,
            "family": family,
            "season": number(season),
            "isLatestSeason": is_latest_season,
            "hasHigherSeason": has_higher_season,
            "lastCheckedAt": prop_or(record, "lastCheckedAt", number(0.0)),
            "lastCheckError": prop_or(record, "lastCheckError", Value::String(String::new())),
            "lastEpisodeCheckError": prop_or(record, "lastEpisodeCheckError", Value::String(String::new())),
            "lastSequelCheckError": prop_or(record, "lastSequelCheckError", Value::String(String::new())),
        }));
    }

    // `.sort((a, b) => b.updatedAt - a.updatedAt)`: the comparator returns NaN for values that
    // are not numbers, and V8's TimSort then keeps the original order.
    sort(&mut rows, |a, b| {
        to_number(prop(b, "updatedAt")) - to_number(prop(a, "updatedAt"))
    });

    Ok(Value::Array(rows))
}

/// Port of `recordPlayback` (`src/native/watch-history.js:42`).
pub fn record_playback(
    previous: Value,
    series: Value,
    index: Value,
    time: Value,
    now: Value,
    metadata: Value,
) -> Result<Value, String> {
    if !number_is_integer(&index)
        || to_number(&index) < 1.0
        || !number_is_finite(&time)
        || to_number(&time) < 0.0
    {
        return Err("无效的播放进度".into());
    }
    let index_number = to_number(&index);
    let time_number = to_number(&time);
    // `now = Date.now()`: the default parameter fires only for `undefined`, not for `null`.
    let now = if is_undefined(&now) {
        number(pinned::now_ms())
    } else {
        now
    };
    // `metadata = {}`: the default fires only for `undefined`.
    let metadata = if is_undefined(&metadata) {
        Value::Object(Map::new())
    } else {
        metadata
    };

    // `previous?.knownTotal ?? totalEpisodes(series)`: `??` falls back on nullish, not on falsy.
    let baseline = match optional_chain(&previous, &["knownTotal"]) {
        Some(value) => value.clone(),
        None => number(total_episodes(&series)),
    };
    let baseline_number = to_number(&baseline);

    let duration = optional_chain(&metadata, &["duration"]).map(to_number);
    let ended = optional_chain(&metadata, &["ended"]) == Some(&Value::Bool(true));
    let completed = ended
        && duration.is_some_and(f64::is_finite)
        && duration.is_some_and(|d| d > 0.0)
        && duration.is_some_and(|d| time_number >= d - js_min(1.0, d * 0.01))
        && duration.is_some_and(|d| time_number <= d + 1.0)
        && index_number == js_max(total_episodes(&series), baseline_number);

    let mut out = spread_object(&previous);
    out.insert("vid_index".into(), index);
    out.insert("currentTime".into(), time);
    out.insert("updatedAt".into(), now);
    out.insert("watched".into(), Value::Bool(true));
    if completed {
        let previous_completed = match optional_chain(&previous, &["completedEpisode"]) {
            Some(value) if number_is_integer(value) => to_number(value),
            _ => 0.0,
        };
        out.insert(
            "completedEpisode".into(),
            number(js_max(previous_completed, index_number)),
        );
    }
    out.insert("knownTotal".into(), baseline.clone());
    let acknowledged = match optional_chain(&previous, &["acknowledgedTotal"]) {
        Some(value) => value.clone(),
        None => baseline,
    };
    out.insert("acknowledgedTotal".into(), acknowledged);
    Ok(Value::Object(out))
}

/// Port of `applyEpisodeUpdate` (`src/native/watch-history.js:54`).
pub fn apply_episode_update(
    record: Value,
    old_series: Value,
    fresh_series: Value,
    now: Value,
) -> Result<Value, String> {
    if is_nullish(&record) {
        return Err(null_property_error("knownTotal", &record));
    }
    // `now = Date.now()`: the default parameter fires only for `undefined`, not for `null`.
    let now = if is_undefined(&now) {
        number(pinned::now_ms())
    } else {
        now
    };

    let baseline = js_max(
        number_or_zero(prop(&record, "knownTotal")),
        total_episodes(&old_series),
    );
    let known_total = js_max(baseline, total_episodes(&fresh_series));
    let acknowledged_total = match optional_chain(&record, &["acknowledgedTotal"]) {
        Some(value) => to_number(value),
        None => baseline,
    };

    let mut out = spread_object(&record);
    out.insert("knownTotal".into(), number(known_total));
    out.insert("acknowledgedTotal".into(), number(acknowledged_total));
    out.insert(
        "unreadEpisodes".into(),
        number(js_max(0.0, known_total - acknowledged_total)),
    );
    out.insert("lastCheckedAt".into(), now);
    out.insert("lastEpisodeCheckError".into(), Value::String(String::new()));
    out.insert(
        "lastCheckError".into(),
        prop_or(
            &record,
            "lastSequelCheckError",
            Value::String(String::new()),
        ),
    );
    Ok(Value::Object(out))
}

/// Port of `seasonTitle` (`src/native/watch-history.js:64`).
pub fn season_title(title: Value) -> Result<Value, String> {
    let parsed = hongguo_core::season_title(&title);
    let mut out = Map::new();
    out.insert("base".into(), Value::String(parsed.base));
    out.insert("season".into(), number(parsed.season));
    out.insert("explicit".into(), Value::Bool(parsed.explicit));
    if parsed.invalid {
        out.insert("invalid".into(), Value::Bool(true));
    }
    Ok(Value::Object(out))
}

/// Port of `sourceOf` (`src/native/watch-history.js:85`).
pub fn source_of(id: Value) -> Result<Value, String> {
    Ok(match hongguo_core::source_of_value(&id) {
        Some(source) => Value::String(source.name().into()),
        None => Value::Null,
    })
}

/// Port of `familyKey` (`src/native/watch-history.js:92`).
pub fn family_key(series: Value) -> Result<Value, String> {
    Ok(Value::String(family_key_value(&series)))
}

/// Port of `seasonOf` (`src/native/watch-history.js:99`).
pub fn season_of(series: Value) -> Result<Value, String> {
    Ok(number(season_of_value(&series)))
}

/// `findSequels(series, candidates, afterSeason)`: the internal form.
fn find_sequels_value(series: &Value, candidates: &Value, after_season: f64) -> Vec<Value> {
    let current = hongguo_core::season_title(prop(series, "series_title"));
    // `String(series.series_id)`: a missing id reads as the text "undefined".
    let series_id = js_string(prop(series, "series_id"));
    let Some(source) = hongguo_core::source_of(&series_id) else {
        return Vec::new();
    };
    if current.base.is_empty() || current.invalid {
        return Vec::new();
    }
    let mut seen: Vec<String> = vec![series_id];

    let mut matched: Vec<Value> = Vec::new();
    for candidate in array_or_empty(candidates) {
        let id = string_or_empty(prop(&candidate, "series_id"));
        let next = hongguo_core::season_title(prop(&candidate, "series_title"));
        if seen.contains(&id)
            || hongguo_core::source_of(&id) != Some(source)
            || !next.explicit
            || next.invalid
            || next.base != current.base
            || next.season <= js_max(current.season, after_season)
        {
            continue;
        }
        seen.push(id);
        matched.push(candidate);
    }

    // `.sort((a, b) => seasonTitle(a.series_title).season - seasonTitle(b.series_title).season)`
    sort(&mut matched, |a, b| {
        hongguo_core::season_title(prop(a, "series_title")).season
            - hongguo_core::season_title(prop(b, "series_title")).season
    });

    matched
        .into_iter()
        .take(10)
        .map(|candidate| {
            json!({
                "series_id": js_string(prop(&candidate, "series_id")),
                "series_title": js_string(prop(&candidate, "series_title")),
            })
        })
        .collect()
}

/// Port of `findSequels` (`src/native/watch-history.js:105`).
pub fn find_sequels(
    series: Value,
    candidates: Value,
    after_season: Option<f64>,
) -> Result<Value, String> {
    if is_nullish(&series) {
        return Err(null_property_error("series_title", &series));
    }
    Ok(Value::Array(find_sequels_value(
        &series,
        &candidates,
        after_season.unwrap_or(0.0),
    )))
}

/// Port of `sequelKeyword` (`src/native/watch-history.js:120`).
pub fn sequel_keyword(series: Value) -> Result<Value, String> {
    // `series.series_title` only throws on a nullish base; on any other value it reads
    // `undefined`, which `seasonTitle` turns into an empty base.
    if is_nullish(&series) {
        return Err(null_property_error("series_title", &series));
    }
    Ok(Value::String(
        hongguo_core::season_title(prop(&series, "series_title")).base,
    ))
}

/// `findSimilarSeries(series, candidates)`: the internal form.
fn find_similar_series_value(series: &Value, candidates: &[Value]) -> Vec<Value> {
    let current = hongguo_core::season_title(prop(series, "series_title"));
    let family = family_key_value(series);
    if family.is_empty() {
        return Vec::new();
    }
    let title = normalize_title(&current.base);
    // `String(series.series_id)`: a missing id reads as the text "undefined".
    let source = hongguo_core::source_of_value(prop(series, "series_id"));
    let mut seen: Vec<String> = vec![js_string(prop(series, "series_id"))];
    let mut results: Vec<Value> = Vec::new();

    // `latest` is a Map keyed by the id: a later candidate with the same id replaces an
    // earlier one but keeps the first insertion position.
    let mut latest: Vec<(String, Value)> = Vec::new();
    for candidate in candidates.iter().cloned() {
        let id = string_or_empty(prop(&candidate, "series_id"));
        match latest.iter().position(|(k, _)| *k == id) {
            Some(index) => latest[index].1 = candidate,
            None => latest.push((id, candidate)),
        }
    }

    for (id, candidate) in &latest {
        let next = hongguo_core::season_title(prop(candidate, "series_title"));
        if seen.contains(id)
            || hongguo_core::source_of(id) != source
            || next.invalid
            || family_key_value(candidate) == family
        {
            continue;
        }
        if !similar_titles(&title, &normalize_title(&next.base)) {
            continue;
        }
        seen.push(id.clone());
        results.push(json!({
            "series_id": id,
            "series_title": js_string(prop(candidate, "series_title")),
        }));
        if results.len() == 5 {
            break;
        }
    }
    results
}

/// Port of `findSimilarSeries` (`src/native/watch-history.js:139`).
pub fn find_similar_series(series: Value, candidates: Value) -> Result<Value, String> {
    Ok(Value::Array(find_similar_series_value(
        &series,
        &array_or_empty(&candidates),
    )))
}

/// Port of `acknowledge` (`src/native/watch-history.js:157`).
pub fn acknowledge(record: Value, snapshot: Value) -> Result<Value, String> {
    if is_nullish(&record) {
        return Err(null_property_error("knownTotal", &record));
    }
    // `record.knownTotal || 0`: the raw value stays as it is.
    let known_total = prop_or(&record, "knownTotal", number(0.0));
    let known_total_number = to_number(&known_total);
    let has_snapshot = truthy(&snapshot);
    let visible_total = if has_snapshot {
        number_or_zero(prop(&snapshot, "total"))
    } else {
        known_total_number
    };
    let acknowledged_total = js_max(
        number_or_zero(prop(&record, "acknowledgedTotal")),
        js_min(known_total_number, visible_total),
    );

    let known_ids: Vec<Value> = array_or_empty(prop(&record, "sequels"))
        .iter()
        .map(|item| prop(item, "series_id").clone())
        .collect();
    let visible_ids: Vec<Value> = if has_snapshot {
        array_or_empty(prop(&snapshot, "sequelIds"))
    } else {
        known_ids.clone()
    };

    // `[...new Set([...(record.seenSequelIds || []), ...visibleIds.filter(id => knownIds.has(id))])]`
    let mut seen: Vec<Value> = Vec::new();
    for id in array_or_empty(prop(&record, "seenSequelIds"))
        .into_iter()
        .chain(visible_ids)
    {
        let known = known_ids.iter().any(|value| same_value_zero(value, &id));
        if known && !seen.iter().any(|value| same_value_zero(value, &id)) {
            seen.push(id);
        }
    }

    let mut out = spread_object(&record);
    out.insert("acknowledgedTotal".into(), number(acknowledged_total));
    out.insert(
        "unreadEpisodes".into(),
        number(js_max(0.0, known_total_number - acknowledged_total)),
    );
    out.insert("seenSequelIds".into(), Value::Array(seen));
    Ok(Value::Object(out))
}

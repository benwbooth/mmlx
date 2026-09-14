// castle_audio_core/src/macros.rs

use crate::types::{Note, ParamValue};
use once_cell::sync::Lazy;
use std::collections::HashSet;

// --- Constants for Parameter Validation ---
static VALID_PARAM_KEYS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "tempo",
        "time_beat",
        "time_note",
        "patch",
        "duty",
        "key",
        "macro", // Key to set/unset the active macro definition
        "velocity",
        "volume",
        "note_volume",
        "attack_envelope",
        "sustain_envelope",
        "release_envelope",
        "release_duration",
        "noise_type",
        "envelope_preset",
        "accidental_suffix", // Internal key
        "transpose",         // Add transpose key
        "instrument",        // Add instrument key
        "pitch",
        "note", // Add pitch and note keys for envelope targets
        "gate", // Gate parameter
    ]
    .iter()
    .cloned()
    .collect()
});

// Define valid targets for envelopes
pub static VALID_ENV_TARGETS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "volume",
        "pitch",
        "duty",
        "note",
        "note_volume",
        "tempo",
        "velocity",
        "transpose",
        "gate",
        "attack_envelope",
        "sustain_envelope",
        "release_envelope",
    ]
    .iter()
    .cloned()
    .collect()
});

// Renamed from VALID_WAVEFORMS
static VALID_PATCHES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    ["sine", "square", "triangle", "sawtooth", "noise"]
        .iter()
        .cloned()
        .collect()
});

static VALID_NOISE_TYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    ["white", "periodic", "brown", "pink"]
        .iter()
        .cloned()
        .collect()
});

static VALID_ENVELOPE_PRESETS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| ["percussion"].iter().cloned().collect());

// Keys whose string values should also be prefix-matched
static STRING_VALUE_KEYS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    ["patch", "noise_type", "envelope_preset"]
        .iter()
        .cloned()
        .collect()
});

// --- Helper Functions ---

/// Finds a unique option that starts with the input prefix (case-insensitive).
/// If there's an exact match, returns that first regardless of other prefix matches.
pub fn match_prefix<'a>(
    input: &str,
    options: &'a HashSet<&'static str>,
) -> Result<&'a str, String> {
    if input.is_empty() {
        return Err("Input cannot be empty.".to_string());
    }

    if let Some(&opt) = options.get(input) {
        return Ok(opt);
    }

    let input_lower = input.to_lowercase();
    for &opt in options.iter() {
        if opt.to_lowercase() == input_lower {
            return Ok(opt);
        }
    }

    let matches: Vec<&str> = options
        .iter()
        .filter(|opt| opt.starts_with(&input_lower))
        .cloned()
        .collect();

    match matches.len() {
        1 => Ok(matches[0]),
        0 => Err(format!(
            "Unknown input \"{}\". Valid options starting with \"{}\": {:?}",
            input,
            input_lower,
            options
                .iter()
                .filter(|opt| opt.starts_with(&input_lower))
                .collect::<Vec<_>>()
        )),
        _ => Err(format!(
            "Ambiguous input \"{}\". Possible matches: {:?}",
            input, matches
        )),
    }
}

/// Creates a ParamSetter note, resolving the key and handling value conversion.
pub fn create_param_setter(key_input: &str, value_input: impl Into<ParamValue>) -> Note {
    if let Ok(resolved_key) = match_prefix(key_input, &VALID_PARAM_KEYS) {
        let resolved_key = resolved_key.to_string();
        let mut param_value = value_input.into();

        if let ParamValue::Note(note_box) = &param_value {
            if let Note::Envelope(env) = &**note_box {
                if VALID_ENV_TARGETS.contains(resolved_key.as_str()) {
                    let mut new_env = env.clone();
                    new_env.target = resolved_key.clone();
                    param_value = ParamValue::Envelope(new_env);
                } else {
                    panic!(
                        "Parameter key \"{}\" cannot accept an envelope value. Valid keys for envelopes: {:?}",
                        resolved_key,
                        VALID_ENV_TARGETS.iter().collect::<Vec<_>>()
                    );
                }
            }
        }

        if let ParamValue::String(ref s) = param_value {
            if STRING_VALUE_KEYS.contains(resolved_key.as_str()) {
                let valid_values: &Lazy<HashSet<&'static str>> = match resolved_key.as_str() {
                    "patch" => &VALID_PATCHES,
                    "noise_type" => &VALID_NOISE_TYPES,
                    "envelope_preset" => &VALID_ENVELOPE_PRESETS,
                    _ => unreachable!(
                        "Key '{}' in STRING_VALUE_KEYS but not handled",
                        resolved_key
                    ),
                };
                match match_prefix(s, valid_values) {
                    Ok(resolved_str) => param_value = ParamValue::String(resolved_str.to_string()),
                    Err(err) => panic!(
                        "Parameter Value Error for key \"{}\": {}. Input was \"{}\"",
                        resolved_key, err, s
                    ),
                }
            } else if let Ok(num_val) = s.parse::<f32>() {
                param_value = ParamValue::Number(num_val);
            }
        }

        Note::ParamSetter {
            key: resolved_key,
            value: param_value,
        }
    } else {
        panic!(
            "Invalid parameter key: '{}' is not a standard parameter.",
            key_input
        );
    }
}

/// Helper for note attribute macros to resolve a key and raw string value into a ParamSetter pair
pub fn resolve_key_value(key: &str, value_str: &str) -> (String, ParamValue) {
    let setter_note = create_param_setter(key, value_str);
    if let Note::ParamSetter { key, value } = setter_note {
        (key, value)
    } else {
        panic!(
            "resolve_key_value expected ParamSetter, got {:?}",
            setter_note
        );
    }
}

// --- Macros ---

#[macro_export]
macro_rules! param {
    ($key:ident =) => {
        $crate::create_param_setter(stringify!($key), $crate::ParamValue::Unset)
    };
    ($key:ident = $value:expr) => {
        $crate::create_param_setter(stringify!($key), $value.clone())
    };
    // Allow multiple key-value pairs, each value optional (`key=` unsets).
    ($($key:ident = $($value:expr)?),* $(,)?) => {
        {
            // Collect individual ParamSetter notes
            let mut setters: Vec<$crate::Note> = Vec::new();
            $( setters.push($crate::create_param_setter(stringify!($key), $crate::param_opt!($($value)?))); )*
            // If only one setter, return it directly; otherwise wrap in a Serial sequence
            if setters.len() == 1 {
                setters.pop().unwrap()
            } else {
                $crate::Note::Serial(setters)
            }
        }
    };
}

/// Helper: optional param value → ParamValue (`empty` means Unset).
#[macro_export]
macro_rules! param_opt {
    () => {
        $crate::ParamValue::Unset
    };
    ($v:expr) => {
        $v.clone()
    };
}

#[macro_export]
macro_rules! en {
    ($($point:expr),* $(,)?) => {{
        let mut env_points = Vec::new();
        $(
            let point_str = stringify!($point);
            let mut duration_str: Option<String> = None;
            let mut value_float: Option<f32> = None;

            // Find the split point: the first char that could start the value part
            // (a digit, '.', or '-') *after* the initial duration code part.
            let mut split_idx: Option<usize> = None;
            let mut first_char_type: Option<&str> = None; // "metric", "s", "ms", "p"

            // Check for metric note duration codes (w, h, q, etc.)
            let metric_dcs = ["w", "h", "q", "e", "i", "t", "x", "o"];
            for &dc in &metric_dcs {
                // Check for dotted versions first
                let prefix_len = if point_str.starts_with(&format!("{}ddd", dc)) { dc.len() + 3 }
                                else if point_str.starts_with(&format!("{}dd", dc)) { dc.len() + 2 }
                                else if point_str.starts_with(&format!("{}d", dc)) { dc.len() + 1 }
                                else if point_str.starts_with(dc) { dc.len() }
                                else { 0 };

                if prefix_len > 0 && point_str.len() > prefix_len {
                     let rest = &point_str[prefix_len..];
                     if let Some(first_val_char) = rest.chars().next() {
                         if first_val_char.is_digit(10) || first_val_char == '.' || first_val_char == '-' {
                            duration_str = Some(point_str[0..prefix_len].to_string());
                            value_float = rest.parse::<f32>().ok();
                            first_char_type = Some("metric");
                            break;
                         }
                     }
                }
            }

            // Check for time units (s, ms, p) potentially preceded by a number
            if value_float.is_none() {
                for unit in ["ms", "s", "p"] { // Check "ms" first
                     if let Some(unit_idx) = point_str.find(unit) {
                         // Check if the character *after* the unit could start the value
                         let potential_split_point = unit_idx + unit.len();
                         if point_str.len() > potential_split_point {
                             let rest = &point_str[potential_split_point..];
                             if let Some(first_val_char) = rest.chars().next() {
                                 if first_val_char.is_digit(10) || first_val_char == '.' || first_val_char == '-' {
                                     // We found a valid unit followed by a potential value
                                     duration_str = Some(point_str[0..potential_split_point].to_string());
                                     value_float = rest.parse::<f32>().ok();
                                     first_char_type = Some(unit); // Store unit type
                                     break;
                                 }
                             }
                         } else if unit_idx == point_str.len() - unit.len() {
                             // Handle case like "5p" with no explicit value
                             // By default for ADSR:
                             // - First point (attack) should reach 1.0
                             // - Last point (release) should go to 0.0
                             // - Middle points determine sustain level, often around 0.7-0.9

                             // Extract the numeric part before the unit
                             let numeric_part = &point_str[0..unit_idx];
                             if let Ok(num) = numeric_part.parse::<f32>() {
                                 duration_str = Some(point_str.to_string());
                                 // For now, keep the 1.0 default for backward compatibility
                                 // but we should encourage explicit values in the documentation
                                 value_float = Some(1.0); // Default to 1.0 for backward compatibility
                                 first_char_type = Some(unit);
                                 break;
                             }
                         }
                     }
                }
            }

            // --- Construct EnvelopePoint ---
            if let (Some(dur_str), Some(v)) = (duration_str, value_float) {
                 let dur = match first_char_type {
                     Some("s") | Some("ms") | Some("p") => {
                         let unit_len = match dur_str.chars().last().unwrap() { 's' => 1, 'p' => 1, _ => 2 }; // ms=2, s=1, p=1
                         let num_part_str = &dur_str[..dur_str.len() - unit_len];
                         // Default to 1.0 if only unit was given (e.g., "s", "p", "ms")
                         let num_part = num_part_str.parse::<f32>().unwrap_or(1.0);
                         match dur_str.chars().last().unwrap() {
                             's' => $crate::types::Duration::Time(num_part, $crate::types::TimeUnit::Seconds),
                             'p' => $crate::types::Duration::Time(num_part, $crate::types::TimeUnit::Percent),
                             _ => $crate::types::Duration::Time(num_part, $crate::types::TimeUnit::Milliseconds),
                         }
                     },
                     Some("metric") | _ => { // Metric or fallback
                         $crate::types::Duration::Note(dur_str)
                     },
                 };
                 env_points.push($crate::types::EnvelopePoint { value: v, duration: dur });
            } else {
                 // If parsing failed, check if it's just a number (assume default duration 'q')
                 if let Ok(v_only) = point_str.parse::<f32>() {
                     env_points.push($crate::types::EnvelopePoint {
                         value: v_only,
                         duration: $crate::types::Duration::Note("q".to_string())
                     });
                 } else {
                     panic!("Invalid envelope point format: {}", point_str);
                 }
            }
        )*

        // For single-point envelopes, set initial duration to zero and add a small maintain segment
        if env_points.len() == 1 {
            // Ensure the single point holds and then releases correctly
            env_points[0].duration = $crate::types::Duration::Time(0.0, $crate::types::TimeUnit::Seconds);
            let last_value = env_points[0].value;
            let maintain_point = $crate::types::EnvelopePoint {
                value: last_value,
                duration: $crate::types::Duration::Time(0.1, $crate::types::TimeUnit::Seconds),
            };
            env_points.push(maintain_point);
        }
        $crate::Note::Envelope($crate::types::Envelope { target: "".to_string(), points: env_points, interpolation: $crate::types::InterpolationType::Linear })
    }};
}

/// Creates an envelope with linear interpolation (identical to env! macro)
#[macro_export]
macro_rules! linen {
    ($($point:expr),* $(,)?) => {{
        let envelope = $crate::en!($($point),*);
        if let $crate::Note::Envelope(mut env) = envelope {
            env.interpolation = $crate::types::InterpolationType::Linear;
            $crate::Note::Envelope(env)
        } else {
            envelope // Should never happen, but return original just in case
        }
    }};
}

/// Creates an envelope with cosine interpolation for smoother transitions
#[macro_export]
macro_rules! cosen {
    ($($point:expr),* $(,)?) => {{
        let envelope = $crate::en!($($point),*);
        if let $crate::Note::Envelope(mut env) = envelope {
            env.interpolation = $crate::types::InterpolationType::Cosine;
            $crate::Note::Envelope(env)
        } else {
            envelope // Should never happen, but return original just in case
        }
    }};
}

/// Creates an envelope with exponential interpolation for sharper or more gradual transitions
#[macro_export]
macro_rules! expen {
    ($($point:expr),* $(,)?) => {{
        let envelope = $crate::en!($($point),*);
        if let $crate::Note::Envelope(mut env) = envelope {
            env.interpolation = $crate::types::InterpolationType::Exponential;
            $crate::Note::Envelope(env)
        } else {
            envelope // Should never happen, but return original just in case
        }
    }};
}

/// Creates an envelope with cubic interpolation for smoother, more natural transitions
#[macro_export]
macro_rules! cuben {
    ($($point:expr),* $(,)?) => {{
        let envelope = $crate::en!($($point),*);
        if let $crate::Note::Envelope(mut env) = envelope {
            env.interpolation = $crate::types::InterpolationType::Cubic;
            $crate::Note::Envelope(env)
        } else {
            envelope // Should never happen, but return original just in case
        }
    }};
}

#[macro_export]
macro_rules! repeat {
    ($count:expr) => {
        $crate::Note::RepeatMarker($count)
    };
}

#[macro_export]
macro_rules! comment {
   ($($arg:tt)*) => {
       $crate::Note::Comment(format!($($arg)*))
   };
}

// --- mmlx canonical aliases (CP437 compat) ---

/// Canonical envelope constructor (alias of `en!`).
#[macro_export]
macro_rules! env {
    ($($point:expr),* $(,)?) => { $crate::en!($($point),*) };
}

/// Canonical interpolation variants.
#[macro_export]
macro_rules! linenv {
    ($($point:expr),* $(,)?) => { $crate::linen!($($point),*) };
}
#[macro_export]
macro_rules! cosenv {
    ($($point:expr),* $(,)?) => { $crate::cosen!($($point),*) };
}
#[macro_export]
macro_rules! expenv {
    ($($point:expr),* $(,)?) => { $crate::expen!($($point),*) };
}
#[macro_export]
macro_rules! cubenv {
    ($($point:expr),* $(,)?) => { $crate::cuben!($($point),*) };
}

// --- Composition operator wrappers (functions exist in types.rs) ---

/// `ser!([...])` ≡ `ser([...])`: sequential composition.
#[macro_export]
macro_rules! ser {
    ($items:expr) => {
        $crate::ser($items)
    };
}
/// `par!([...])` ≡ `par([...])`: parallel composition (max duration).
#[macro_export]
macro_rules! par {
    ($items:expr) => {
        $crate::par($items)
    };
}
/// `parmin!([...])` ≡ `parmin([...])`: parallel composition (min duration).
#[macro_export]
macro_rules! parmin {
    ($items:expr) => {
        $crate::parmin($items)
    };
}
/// `forkseq!([...])` ≡ `forkseq([...])`: forked serial (advances 0).
#[macro_export]
macro_rules! forkseq {
    ($items:expr) => {
        $crate::forkseq($items)
    };
}
/// `forkser!` is the CP437-spelled alias of `forkseq!`.
#[macro_export]
macro_rules! forkser {
    ($items:expr) => {
        $crate::forkseq($items)
    };
}
/// `forkpar!([...])` ≡ `forkpar([...])`: forked parallel (advances 0).
#[macro_export]
macro_rules! forkpar {
    ($items:expr) => {
        $crate::forkpar($items)
    };
}

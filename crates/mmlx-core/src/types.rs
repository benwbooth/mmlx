use async_recursion::async_recursion;
use genawaiter::sync::{Co, Gen};
use linked_hash_map::LinkedHashMap;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Enum representing possible parameter value types
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ParamValue {
    String(String),
    Number(f32),
    Note(Box<Note>),
    Envelope(Envelope),
    Unset,
}

impl std::fmt::Display for ParamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamValue::String(s) => write!(f, "\\\"{}\\\"", s),
            ParamValue::Number(n) => write!(f, "{}", n),
            ParamValue::Note(n) => write!(f, "{}", n),
            ParamValue::Envelope(e) => write!(f, "{}", e),
            ParamValue::Unset => write!(f, "<unset>"),
        }
    }
}

impl ParamValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            ParamValue::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_number(&self) -> Option<f32> {
        match self {
            ParamValue::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_note(&self) -> Option<&Note> {
        match self {
            ParamValue::Note(n) => Some(&**n),
            _ => None,
        }
    }
    pub fn as_envelope(&self) -> Option<&Envelope> {
        match self {
            ParamValue::Envelope(e) => Some(e),
            _ => None,
        }
    }
}

// From implementations
impl From<f32> for ParamValue {
    fn from(v: f32) -> Self {
        ParamValue::Number(v)
    }
}
impl From<String> for ParamValue {
    fn from(v: String) -> Self {
        ParamValue::String(v)
    }
}
impl From<&str> for ParamValue {
    fn from(v: &str) -> Self {
        ParamValue::String(v.to_string())
    }
}
impl From<i32> for ParamValue {
    fn from(v: i32) -> Self {
        ParamValue::Number(v as f32)
    }
}
impl From<Note> for ParamValue {
    fn from(v: Note) -> Self {
        ParamValue::Note(Box::new(v))
    }
}
impl From<Envelope> for ParamValue {
    fn from(v: Envelope) -> Self {
        ParamValue::Envelope(v)
    }
}

/// Time unit for specifying durations in envelopes
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TimeUnit {
    /// Duration in seconds
    Seconds,
    /// Duration in milliseconds
    Milliseconds,
    /// Duration as percentage of the note's total duration (0.0 to 100.0)
    Percent,
}

/// Duration representation for envelope points and notes
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Duration {
    Note(String),
    Time(f32, TimeUnit),
}

/// A point in an envelope curve with a value and duration
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvelopePoint {
    pub value: f32,
    pub duration: Duration,
}

/// Defines the interpolation method used between envelope points
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InterpolationType {
    /// Linear interpolation between points (default)
    Linear,
    /// Cosine interpolation for smoother transitions
    Cosine,
    /// Exponential interpolation for sharper or more gradual transitions
    Exponential,
    /// Cubic interpolation for smoother, more natural transitions
    Cubic,
}

/// An envelope defines a series of values over time for modulating parameters
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub target: String,
    pub points: Vec<EnvelopePoint>,
    pub interpolation: InterpolationType,
}

/// A musical note or collection: atomic, serial melody, or parallel chord.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Note {
    Atom {
        midi: u8,
        duration: f32,
        parameters: Vec<(String, ParamValue)>,
    },
    AtomImplicitDuration {
        midi: u8,
        parameters: Vec<(String, ParamValue)>,
    },
    Rest {
        duration: f32,
        parameters: Vec<(String, ParamValue)>,
    },
    DurationTie {
        duration: f32,
    },
    PreviousPitch {
        optional_duration: Option<f32>,
        parameters: Vec<(String, ParamValue)>,
    },
    Serial(Vec<Note>),
    Parallel(Vec<Note>),
    ParamSetter {
        key: String,
        value: ParamValue,
    },
    Envelope(Envelope),
    RepeatMarker(usize),
    ParallelMin(Vec<Note>),
    ForkSequence(Vec<Note>),
    ForkParallel(Vec<Note>),
    Comment(String),
}

impl Duration {
    pub fn to_seconds(&self, tempo: f32, time_note: u8, total_duration: Option<f32>) -> f32 {
        match self {
            Duration::Note(note_duration) => {
                let base_duration = match note_duration.as_str() {
                    "w" => 1.0,
                    "h" => 0.5,
                    "q" => 0.25,
                    "e" => 0.125,
                    "i" => 0.0625,
                    "t" => 0.03125,
                    "x" => 0.015625,
                    "o" => 0.0078125,
                    "wd" => 1.5,
                    "hd" => 0.75,
                    "qd" => 0.375,
                    "ed" => 0.1875,
                    "id" => 0.09375,
                    "td" => 0.046875,
                    "xd" => 0.0234375,
                    "od" => 0.01171875,
                    _ => 0.25, // Default to quarter note
                };
                let seconds_per_beat = 60.0 / tempo;
                let beats_per_whole_note = 4.0;
                let beats_per_quarter = 4.0 / time_note as f32;
                let duration_secs =
                    base_duration * beats_per_whole_note * beats_per_quarter * seconds_per_beat;
                duration_secs
            }
            Duration::Time(value, unit) => match unit {
                TimeUnit::Seconds => *value,
                TimeUnit::Milliseconds => *value / 1000.0,
                TimeUnit::Percent => {
                    if let Some(total) = total_duration {
                        total * (*value / 100.0)
                    } else {
                        0.0
                    }
                }
            },
        }
    }
}

impl EnvelopePoint {
    pub fn new_note(value: f32, note_duration: &str) -> Self {
        EnvelopePoint {
            value,
            duration: Duration::Note(note_duration.to_string()),
        }
    }
    pub fn new_seconds(value: f32, seconds: f32) -> Self {
        EnvelopePoint {
            value,
            duration: Duration::Time(seconds, TimeUnit::Seconds),
        }
    }
    pub fn new_ms(value: f32, ms: f32) -> Self {
        EnvelopePoint {
            value,
            duration: Duration::Time(ms, TimeUnit::Milliseconds),
        }
    }
    pub fn new_percent(value: f32, percent: f32) -> Self {
        EnvelopePoint {
            value,
            duration: Duration::Time(percent, TimeUnit::Percent),
        }
    }
}

impl Envelope {
    pub fn with_points(target: &str, points: Vec<EnvelopePoint>) -> Self {
        Envelope {
            target: target.to_string(),
            points,
            interpolation: InterpolationType::Linear, // Default to linear interpolation
        }
    }

    /// Create a new envelope with the specified interpolation type
    pub fn with_interpolation(
        target: &str,
        points: Vec<EnvelopePoint>,
        interpolation: InterpolationType,
    ) -> Self {
        Envelope {
            target: target.to_string(),
            points,
            interpolation,
        }
    }

    /// Evaluate the envelope at a specific time point.
    pub fn evaluate(
        &self,
        time: f32,
        tempo: f32,
        time_note: u8,
        total_duration: Option<f32>,
    ) -> f32 {
        if self.points.is_empty() {
            debug!("Empty envelope for target '{}', returning 0.0", self.target);
            return 0.0;
        }

        if self.points.len() == 1 {
            debug!(
                "Single-point envelope for target '{}', returning {}",
                self.target, self.points[0].value
            );
            return self.points[0].value;
        }

        // Ensure total_duration is available, especially for percentage points
        let total_dur = match total_duration {
            Some(d) if d > 1e-9 => d, // Use provided duration if positive
            _ => {
                // Attempt to calculate total duration if not provided or zero
                warn!("Envelope::evaluate called without valid total_duration for target '{}', calculating approx duration. Percentage points will be inaccurate.", self.target);
                let mut calculated_duration = 0.0;
                for i in 1..self.points.len() {
                    match self.points[i].duration {
                        Duration::Time(_, TimeUnit::Percent) => { /* Ignore percent */ }
                        _ => {
                            calculated_duration +=
                                self.points[i].duration.to_seconds(tempo, time_note, None)
                        }
                    }
                }
                if calculated_duration < 1e-9 {
                    1.0
                } else {
                    calculated_duration
                } // Fallback to 1s if still zero
            }
        };

        // Calculate absolute time for each point
        let mut point_times: Vec<(f32, f32)> = Vec::with_capacity(self.points.len());
        let mut cumulative_time = 0.0;

        for (i, point) in self.points.iter().enumerate() {
            let point_time = match &point.duration {
                Duration::Time(t, TimeUnit::Seconds) => {
                    if i == 0 {
                        0.0
                    } else {
                        cumulative_time + t
                    }
                }
                Duration::Time(t, TimeUnit::Milliseconds) => {
                    if i == 0 {
                        0.0
                    } else {
                        cumulative_time + (t / 1000.0)
                    }
                }
                Duration::Time(p, TimeUnit::Percent) => (p / 100.0) * total_dur,
                Duration::Note(_n) => {
                    if i == 0 {
                        0.0
                    } else {
                        cumulative_time
                            + point.duration.to_seconds(tempo, time_note, Some(total_dur))
                    }
                }
            };
            point_times.push((point_time, point.value));
            cumulative_time = point_time; // Update cumulative time for next iteration
        }

        // Sort points by time for percentages that might cause out-of-order definitions
        if point_times.len() > 1 {
            point_times[1..]
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        }

        // Find surrounding points and interpolate
        if time <= point_times[0].0 + 1e-6 {
            return point_times[0].1;
        }

        for i in 0..(point_times.len() - 1) {
            let (prev_time, prev_val) = point_times[i];
            let (next_time, next_val) = point_times[i + 1];

            // Check if time falls within this segment
            if time >= prev_time - 1e-6 && time <= next_time + 1e-6 {
                let segment_duration = next_time - prev_time;

                if segment_duration < 1e-6 {
                    return if (time - next_time).abs() < (time - prev_time).abs() {
                        next_val
                    } else {
                        prev_val
                    };
                }

                // Calculate interpolation factor
                let t = (time - prev_time) / segment_duration;
                let t_clamped = t.clamp(0.0, 1.0);

                // Apply interpolation based on the envelope's interpolation type
                match self.interpolation {
                    InterpolationType::Linear => {
                        // Linear interpolation (default)
                        return prev_val + t_clamped * (next_val - prev_val);
                    }
                    InterpolationType::Cosine => {
                        // Cosine interpolation
                        let cosine_t = (1.0 - f32::cos(t_clamped * std::f32::consts::PI)) * 0.5;
                        return prev_val + cosine_t * (next_val - prev_val);
                    }
                    InterpolationType::Exponential => {
                        // Exponential interpolation
                        // Handle special case for negative values or zero
                        if prev_val <= 0.0 || next_val <= 0.0 {
                            // Fall back to linear interpolation for non-positive values
                            return prev_val + t_clamped * (next_val - prev_val);
                        }

                        // Perform exponential interpolation using power function
                        let exp_t = t_clamped * t_clamped;
                        return prev_val * f32::powf(next_val / prev_val, exp_t);
                    }
                    InterpolationType::Cubic => {
                        // Cubic interpolation (smoothstep)
                        let cubic_t = t_clamped * t_clamped * (3.0 - 2.0 * t_clamped);
                        return prev_val + cubic_t * (next_val - prev_val);
                    }
                }
            }
        }

        // If time is after the last point, return the last point's value
        point_times.last().unwrap().1
    }
}

// Display and Debug implementations
impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Note::Atom {
                midi,
                duration,
                parameters,
                ..
            } => {
                let (note_letter, accidental) = match midi % 12 {
                    0 => ("c", ""),
                    1 => ("c", "s"),
                    2 => ("d", ""),
                    3 => ("d", "s"),
                    4 => ("e", ""),
                    5 => ("f", ""),
                    6 => ("f", "s"),
                    7 => ("g", ""),
                    8 => ("g", "s"),
                    9 => ("a", ""),
                    10 => ("a", "s"),
                    11 => ("b", ""),
                    _ => unreachable!(),
                };
                let octave = (*midi / 12).saturating_sub(1);
                let duration_name = get_duration_symbol(*duration);
                write!(
                    f,
                    "{}{}{}{}",
                    note_letter, accidental, octave, duration_name
                )?;
                if !parameters.is_empty() {
                    write!(f, "!(")?;
                    for (idx, (key, value)) in parameters.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        // Format based on ParamValue type
                        match value {
                            ParamValue::String(s) => write!(f, "{}=\"{}\"", key, s)?,
                            ParamValue::Number(n) => write!(f, "{}={}", key, n)?,
                            _ => write!(f, "{}={:?}", key, value)?,
                        }
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Note::AtomImplicitDuration { midi, parameters } => {
                let (note_letter, accidental) = match midi % 12 {
                    0 => ("c", ""),
                    1 => ("c", "s"),
                    2 => ("d", ""),
                    3 => ("d", "s"),
                    4 => ("e", ""),
                    5 => ("f", ""),
                    6 => ("f", "s"),
                    7 => ("g", ""),
                    8 => ("g", "s"),
                    9 => ("a", ""),
                    10 => ("a", "s"),
                    11 => ("b", ""),
                    _ => unreachable!(),
                };
                let octave = (*midi / 12).saturating_sub(1);
                write!(f, "{}{}{}", note_letter, accidental, octave)?;
                if !parameters.is_empty() {
                    write!(f, "!(")?;
                    for (idx, (key, value)) in parameters.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        match value {
                            ParamValue::String(s) => write!(f, "{}=\"{}\"", key, s)?,
                            ParamValue::Number(n) => write!(f, "{}={}", key, n)?,
                            _ => write!(f, "{}={:?}", key, value)?,
                        }
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Note::Rest {
                duration,
                parameters,
            } => {
                let duration_name = get_duration_symbol(*duration);
                write!(f, "r{}", duration_name)?;
                if !parameters.is_empty() {
                    write!(f, "!(")?;
                    for (idx, (key, value)) in parameters.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        match value {
                            ParamValue::String(s) => write!(f, "{}=\"{}\"", key, s)?,
                            ParamValue::Number(n) => write!(f, "{}={}", key, n)?,
                            _ => write!(f, "{}={:?}", key, value)?,
                        }
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Note::Serial(notes) => {
                if f.alternate() {
                    writeln!(f, "ser([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        let formatted = format!("{:#}", note);
                        for line in formatted.lines() {
                            writeln!(f, "    {}", line)?;
                        }
                        if idx < notes.len() - 1 {
                            writeln!(f, "    ,")?;
                        }
                    }
                    write!(f, "])")
                } else {
                    write!(f, "ser([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", note)?;
                    }
                    write!(f, "])")
                }
            }
            Note::Parallel(notes) => {
                if f.alternate() {
                    writeln!(f, "par([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        let formatted = format!("{:#}", note);
                        for line in formatted.lines() {
                            writeln!(f, "    {}", line)?;
                        }
                        if idx < notes.len() - 1 {
                            writeln!(f, "    ,")?;
                        }
                    }
                    write!(f, "])")
                } else {
                    write!(f, "par([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", note)?;
                    }
                    write!(f, "])")
                }
            }
            Note::ParamSetter { key, value } => match value {
                ParamValue::String(s) => write!(f, "param!({}=\"{}\")", key, s),
                ParamValue::Number(n) => write!(f, "param!({}={})", key, n),
                _ => write!(f, "param!({}={:?})", key, value),
            },
            Note::Envelope(env) => {
                write!(f, "{}", env)
            }
            Note::DurationTie { duration } => {
                write!(f, "(tie:{})", get_duration_symbol(*duration))
            }
            Note::PreviousPitch {
                optional_duration,
                parameters,
            } => {
                write!(f, "p")?;
                if let Some(dur) = optional_duration {
                    write!(f, "{}", get_duration_symbol(*dur))?;
                }
                if !parameters.is_empty() {
                    write!(f, "!(")?;
                    for (idx, (key, value)) in parameters.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        match value {
                            ParamValue::String(s) => write!(f, "{}=\"{}\"", key, s)?,
                            ParamValue::Number(n) => write!(f, "{}={}", key, n)?,
                            _ => write!(f, "{}={:?}", key, value)?,
                        }
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Note::RepeatMarker(count) => {
                write!(f, "(repeat:{})", count)
            }
            Note::ParallelMin(notes) => {
                if f.alternate() {
                    writeln!(f, "parmin([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        let formatted = format!("{:#}", note);
                        for line in formatted.lines() {
                            writeln!(f, "    {}", line)?;
                        }
                        if idx < notes.len() - 1 {
                            writeln!(f, "    ,")?;
                        }
                    }
                    write!(f, "])")
                } else {
                    write!(f, "parmin([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", note)?;
                    }
                    write!(f, "])")
                }
            }
            Note::ForkSequence(notes) => {
                if f.alternate() {
                    writeln!(f, "forkseq([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        let formatted = format!("{:#}", note);
                        for line in formatted.lines() {
                            writeln!(f, "    {}", line)?;
                        }
                        if idx < notes.len() - 1 {
                            writeln!(f, "    ,")?;
                        }
                    }
                    write!(f, "])")
                } else {
                    write!(f, "forkseq([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", note)?;
                    }
                    write!(f, "])")
                }
            }
            Note::ForkParallel(notes) => {
                if f.alternate() {
                    writeln!(f, "forkpar([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        let formatted = format!("{:#}", note);
                        for line in formatted.lines() {
                            writeln!(f, "    {}", line)?;
                        }
                        if idx < notes.len() - 1 {
                            writeln!(f, "    ,")?;
                        }
                    }
                    write!(f, "])")
                } else {
                    write!(f, "forkpar([")?;
                    for (idx, note) in notes.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", note)?;
                    }
                    write!(f, "])")
                }
            }
            Note::Comment(comment) => {
                write!(f, "// {}", comment)
            }
        }
    }
}

impl std::fmt::Debug for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

// Key signature helpers
pub fn get_base_midi_and_offset(midi: u8) -> (u8, i8) {
    let base = midi / 12 * 12;
    let offset = midi % 12;
    let natural_offset = match offset {
        0 | 2 | 4 | 5 | 7 | 9 | 11 => 0,
        1 | 3 | 6 | 8 | 10 => 1, // Represents a sharp/flat position
        _ => unreachable!(),
    };
    (base + natural_offset, offset as i8 - natural_offset as i8)
}

pub fn get_key_signature_adjustment(key_str: &str, natural_pitch_class: u8) -> i8 {
    let (_root, mode) = parse_key_signature(key_str);
    let scale_intervals = match mode {
        0 => &[0, 2, 4, 5, 7, 9, 11], // Major
        _ => &[0, 2, 3, 5, 7, 8, 10], // Minor (Natural Minor for simplicity)
    };
    // Check sharps/flats based on C major/A minor reference
    let reference_naturals = [0, 2, 4, 5, 7, 9, 11]; // C Major scale pitch classes
    let is_natural_in_c = reference_naturals.contains(&natural_pitch_class);
    let is_in_key_scale = scale_intervals.contains(&natural_pitch_class);

    if is_natural_in_c && !is_in_key_scale {
        -1
    }
    // Natural in C, but flat in key
    else if !is_natural_in_c && is_in_key_scale {
        1
    }
    // Sharp/Flat in C, but naturalized in key (e.g. F# in Gmaj)
    else {
        0
    } // No change needed
}

pub fn parse_key_signature(key_str: &str) -> (u8, i8) {
    // Simple parser, assumes format like "Cmaj", "amin", "Bbmaj", "f#min"
    let key_lower = key_str.to_lowercase();
    let mut _root_note = 0u8;
    let mut mode = 0i8; // 0 for major, -1 for minor
    let mut cursor = 0;

    match key_lower.chars().nth(0) {
        Some('c') => _root_note = 0,
        Some('d') => _root_note = 2,
        Some('e') => _root_note = 4,
        Some('f') => _root_note = 5,
        Some('g') => _root_note = 7,
        Some('a') => _root_note = 9,
        Some('b') => _root_note = 11,
        _ => panic!("Invalid key signature root: {}", key_str),
    }
    cursor += 1;

    match key_lower.chars().nth(1) {
        Some('#') | Some('s') => {
            _root_note = (_root_note + 1) % 12;
            cursor += 1;
        }
        Some('b') | Some('f') => {
            _root_note = (_root_note + 11) % 12;
            cursor += 1;
        }
        _ => { /* No accidental */ }
    }

    if key_lower[cursor..].starts_with("maj") {
        mode = 0;
    } else if key_lower[cursor..].starts_with("min") {
        mode = -1;
    } else if key_lower.ends_with('m') && !key_lower.ends_with("maj") {
        mode = -1;
    } // Default to major if no suffix

    (_root_note, mode)
}

// Helper for displaying duration
pub fn get_duration_symbol(duration: f32) -> String {
    match duration {
        1.5 => "wd".to_string(),
        1.0 => "w".to_string(),
        0.75 => "hd".to_string(),
        0.5 => "h".to_string(),
        0.375 => "qd".to_string(),
        0.25 => "q".to_string(),
        0.1875 => "ed".to_string(),
        0.125 => "e".to_string(),
        0.09375 => "id".to_string(),
        0.0625 => "i".to_string(),
        0.046875 => "td".to_string(),
        0.03125 => "t".to_string(),
        0.0234375 => "xd".to_string(),
        0.015625 => "x".to_string(),
        0.01171875 => "od".to_string(),
        0.0078125 => "o".to_string(),
        _ => format!("{:.4}", duration), // Fallback for unusual durations
    }
}

// Display implementation for Envelope
impl std::fmt::Display for Envelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use the appropriate macro name based on interpolation type
        let macro_name = match self.interpolation {
            InterpolationType::Linear => "env",
            InterpolationType::Cosine => "cosenv",
            InterpolationType::Exponential => "expenv",
            InterpolationType::Cubic => "cubenv",
        };

        write!(f, "{}!(", macro_name)?;
        for (i, point) in self.points.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            // Format point duration and value
            match &point.duration {
                Duration::Note(note_dur) => {
                    write!(f, "{}{}", point.value, note_dur)?;
                }
                Duration::Time(value, unit) => match unit {
                    TimeUnit::Seconds => write!(f, "{}{}s", point.value, value)?,
                    TimeUnit::Milliseconds => write!(f, "{}{}ms", point.value, value)?,
                    TimeUnit::Percent => write!(f, "{}{}p", point.value, value)?,
                },
            }
        }
        write!(f, ")")
    }
}

// Unique note IDs generator
static NEXT_NOTE_ID: AtomicU64 = AtomicU64::new(0);

pub fn generate_unique_note_id() -> u64 {
    NEXT_NOTE_ID.fetch_add(1, Ordering::Relaxed)
}

/// Represents musical events generated from the Note structure.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MusicalEventType {
    /// Start playing a note.
    NoteOn {
        note_id: u64, // Unique ID for this note instance
        pitch_midi: u8,
        velocity: f32, // Amplitude/volume (e.g., 0.0 to 1.0)
        parameters: HashMap<String, ParamValue>,

        // New envelope phases for ADSR control
        attack_envelope: Option<Envelope>, // Initial Attack/Decay phase
        sustain_envelope: Option<Envelope>, // Looping Sustain phase (after Attack/Decay completes)
        release_envelope: Option<Envelope>, // Release phase (triggered by NoteOff)

        // Remaining envelopes (for pitch, duty cycle, etc.)
        other_envelopes: Vec<Envelope>,
    },
    /// A period of silence.
    Rest { duration_secs: f32 },
    /// Set a parameter that affects subsequent events.
    SetParameter { key: String, value: ParamValue },
    /// Stop playing a specific note instance identified by its ID.
    NoteOff { note_id: u64 },
    /// Represents a comment to be logged.
    Comment(String),
}

/// An event tuple containing the absolute time and the event type.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimedMusicalEvent {
    /// Absolute time in seconds from the beginning of the sequence.
    pub time_seconds: f32,
    pub real_duration: f32,
    /// The type of musical event.
    pub event: MusicalEventType,
    pub instrument_name: String, // Name of the instrument to use
}

/// Trait for musical instruments that can process events and generate audio.
pub trait Instrument: Send + Sync + 'static {
    /// Process a single musical event at its specified time.
    fn process_event(&mut self, event: &TimedMusicalEvent, rate: usize);

    /// Process an iterator of musical events.
    fn process_events(
        &mut self,
        events: Box<dyn Iterator<Item = TimedMusicalEvent> + Send + Sync>,
        rate: usize,
    );

    /// Generate the next block of audio samples.
    fn generate_samples(&mut self, count: usize, rate: usize) -> Vec<[f32; 2]>;

    /// Check if the instrument is currently idle.
    fn is_idle(&self) -> bool {
        false // Default implementation: assume never idle
    }
}

impl Note {
    /// Helper to get the base duration from Atom or Rest, otherwise None.
    pub fn get_base_duration(&self) -> Option<f32> {
        match self {
            Note::Atom { duration, .. } => Some(*duration),
            Note::Rest { duration, .. } => Some(*duration),
            Note::Comment(_) => None, // Comments have no duration
            _ => None,                // All other variants don't have a single base duration
        }
    }

    /// Adds or updates a parameter. Assumes key/value are already resolved/validated.
    /// Value is a ParamValue, allowing envelopes, numbers, strings, etc.
    pub fn param(mut self, key: String, value: ParamValue) -> Self {
        match &mut self {
            Note::Atom { parameters, .. }
            | Note::Rest { parameters, .. }
            | Note::AtomImplicitDuration { parameters, .. } => {
                let mut map: LinkedHashMap<String, ParamValue> = parameters.drain(..).collect();
                map.insert(key, value);
                *parameters = map.into_iter().collect();
            }
            Note::DurationTie { .. } => { /* Ties don't have attributes */ }
            Note::PreviousPitch { parameters, .. } => {
                let mut map: LinkedHashMap<String, ParamValue> = parameters.drain(..).collect();
                map.insert(key, value);
                *parameters = map.into_iter().collect();
            }
            Note::Serial(notes) | Note::Parallel(notes) => {
                *notes = notes
                    .iter()
                    .map(|n| n.clone().param(key.clone(), value.clone()))
                    .collect();
            }
            Note::ParallelMin(notes) | Note::ForkSequence(notes) | Note::ForkParallel(notes) => {
                *notes = notes
                    .iter()
                    .map(|n| n.clone().param(key.clone(), value.clone()))
                    .collect();
            }
            Note::ParamSetter { .. }
            | Note::Envelope(_)
            | Note::RepeatMarker(_)
            | Note::Comment(_) // Comments don't have parameters
            => { /* No-op for variants without parameters */ }
        }
        self
    }

    /// Recursive helper to generate events via a producer.
    #[async_recursion(Sync)]
    pub async fn generate_events_recursive<'a>(
        note: &'a Note,
        start_time: f32,
        active_params: &mut LinkedHashMap<String, ParamValue>,
        producer: &Co<TimedMusicalEvent>,
    ) -> f32 {
        // Returns end time
        // Helper to get numeric param, checking type and parsing string if needed
        let get_numeric_param =
            |params_map: &LinkedHashMap<String, ParamValue>, key: &str, default: f32| -> f32 {
                match params_map.get(key) {
                    Some(ParamValue::Number(n)) => *n,
                    Some(ParamValue::String(s)) => s.parse::<f32>().unwrap_or(default),
                    _ => default, // Not found, or wrong type
                }
            };
        // Helper to get string param
        let get_string_param =
            |params_map: &LinkedHashMap<String, ParamValue>, key: &str| -> Option<String> {
                match params_map.get(key) {
                    Some(ParamValue::String(s)) => Some(s.clone()),
                    _ => None,
                }
            };

        match note {
            Note::Atom {
                midi,
                duration,
                parameters,
            } => {
                // +++ Log map received +++
                // info!("NoteOn START: midi={}, received active_params={:?}", midi, active_params);
                // +++ End Log +++

                // --- Build final parameters for this note ---
                // Note: Parameters from the macro invocation itself (like `macro=...`) are NOT included
                // in the `parameters` Vec here; they need to be handled by the procedural macro.
                let mut final_params = active_params.clone();
                for (k, v) in parameters {
                    final_params.insert(k.clone(), v.clone());
                }

                // --- Calculate original real_duration FIRST ---
                // Needed for both macro invocation and normal note generation
                let tempo = get_numeric_param(&final_params, "tempo", 60.0);
                let time_note_val = get_numeric_param(&final_params, "time_note", 4.0);
                let seconds_per_beat = 60.0 / tempo;
                let beats_per_whole_note = 4.0;
                let beats_per_quarter = 4.0 / time_note_val;
                let whole_note_duration_secs =
                    beats_per_whole_note * beats_per_quarter * seconds_per_beat;
                let original_real_duration = *duration * whole_note_duration_secs;

                // --- Check for Macro Invocation ---
                if let Some(ParamValue::Note(macro_definition_boxed)) = active_params.get("macro") {
                    // Macro is active, fork its execution
                    let base_note_midi = 60;
                    let offset = *midi as i16 - base_note_midi as i16;
                    let transposed_macro = transpose_note(&macro_definition_boxed, offset as i8);

                    // Clone params for the macro, remove "macro" key from the clone
                    let mut macro_params = active_params.clone();
                    macro_params.remove("macro");

                    debug!("Invoking active macro at time {}", start_time);
                    let _ = Self::generate_events_recursive(
                        &transposed_macro,
                        start_time,
                        &mut macro_params, // Use the cloned, modified params
                        producer,
                    )
                    .await;

                    // Advance timeline by original note duration
                    return start_time + original_real_duration;
                }
                // --- End Macro Invocation Check ---

                // --- Normal Atom Event Generation (if no macro was invoked) ---
                let primary_volume_control = final_params
                    .remove("note_volume")
                    .or_else(|| final_params.remove("adsr"));

                // --- Velocity and Envelope handling starts here ---
                let final_velocity: f32;
                if let Some(control_value) = primary_volume_control {
                    match control_value {
                        ParamValue::Number(num) => {
                            // Use number as primary velocity source (overrides velocity/dynamics)
                            final_velocity = (num / 127.0).clamp(0.0, 1.0);
                        }
                        ParamValue::Envelope(mut env) => {
                            // Use envelope as primary volume control
                            final_velocity = 1.0; // Set base velocity to max, envelope handles shape
                            env.target = "volume".to_string(); // Ensure target is "volume"
                        }
                        _ => {
                            // Invalid type for note_volume/adsr, fall back to velocity (removed dynamics fallback)
                            // Consider adding a warning log here
                            let velocity_param =
                                get_numeric_param(&final_params, "velocity", 100.0); // Default to 100 if velocity absent
                            final_velocity = (velocity_param / 127.0).clamp(0.0, 1.0);
                        }
                    }
                } else {
                    // No note_volume/adsr found, use standard velocity (removed dynamics fallback)
                    let velocity_param = get_numeric_param(&final_params, "velocity", 100.0); // Default to 100 if velocity absent
                    final_velocity = (velocity_param / 127.0).clamp(0.0, 1.0);
                }

                // --- Calculate real_duration (metric) and sounding_duration (gated) ---
                let tempo = get_numeric_param(&final_params, "tempo", 60.0);
                let time_note_val = get_numeric_param(&final_params, "time_note", 4.0);
                let seconds_per_beat = 60.0 / tempo;
                let beats_per_whole_note = 4.0;
                let beats_per_quarter = 4.0 / time_note_val;
                let whole_note_duration_secs =
                    beats_per_whole_note * beats_per_quarter * seconds_per_beat;
                let base_duration = *duration; // Use original duration from Note::Atom
                let real_duration = base_duration * whole_note_duration_secs;

                // --- Apply Gate ---
                let gate_value = get_numeric_param(&final_params, "gate", 1.0);
                let sounding_duration_secs = (real_duration * gate_value).max(0.0); // Ensure non-negative

                // Debug Logging (using final_params)
                // info!(
                //     "NoteOn Calc: midi={}, base_dur={}, tempo={}, time_note={}, whole_dur_s={}, final_params={:?}",
                //     midi,
                //     base_duration,
                //     tempo,
                //     time_note_val,
                //     whole_note_duration_secs,
                //     final_params // Log the map used for calculation
                // );
                if gate_value != 1.0 {
                    debug!(
                        "NoteOn Gate: midi={}, metric_dur={:.4}s, gate={:.2}, sounding_dur={:.4}s",
                        midi, real_duration, gate_value, sounding_duration_secs
                    );
                }

                // --- Get instrument name (using final_params) ---
                let instrument_name = get_string_param(&final_params, "instrument")
                    .unwrap_or_else(|| "basic_synth".to_string());

                // --- Separate Remaining Envelopes and Parameters into NoteOn ADSR/Mod envelopes ---
                let mut note_event_params: HashMap<String, ParamValue> = HashMap::new();
                let mut attack_envelope: Option<Envelope> = None;
                let mut sustain_envelope: Option<Envelope> = None;
                let mut release_envelope: Option<Envelope> = None;
                let mut other_envelopes: Vec<Envelope> = Vec::new();

                // Segregate envelopes by key
                for (key, value) in final_params.into_iter() {
                    match (key.as_str(), value) {
                        ("attack_envelope", ParamValue::Envelope(env)) => {
                            attack_envelope = Some(env)
                        }
                        ("sustain_envelope", ParamValue::Envelope(env)) => {
                            sustain_envelope = Some(env)
                        }
                        ("release_envelope", ParamValue::Envelope(env)) => {
                            release_envelope = Some(env)
                        }
                        (_, ParamValue::Envelope(env)) => other_envelopes.push(env),
                        (k, v) => {
                            note_event_params.insert(k.to_string(), v);
                        }
                    }
                }

                // Create and yield NoteOn event with the new structure
                let note_id = generate_unique_note_id(); // Assign a unique ID and save it
                let event_data = MusicalEventType::NoteOn {
                    note_id,
                    pitch_midi: *midi,
                    velocity: final_velocity, // Use the calculated final_velocity
                    parameters: note_event_params, // Params excluding handled ones
                    attack_envelope,
                    sustain_envelope,
                    release_envelope,
                    other_envelopes,
                };
                producer
                    .yield_(TimedMusicalEvent {
                        time_seconds: start_time,
                        real_duration: original_real_duration,
                        event: event_data,
                        instrument_name: instrument_name.clone(),
                    })
                    .await;

                // Also create a corresponding NoteOff event at the end of the note's duration
                let note_off_event = MusicalEventType::NoteOff { note_id };
                let end_time = start_time + original_real_duration;

                producer
                    .yield_(TimedMusicalEvent {
                        time_seconds: end_time,
                        real_duration: 0.0, // NoteOff has no duration of its own
                        event: note_off_event,
                        instrument_name,
                    })
                    .await;

                end_time // Return the end time
            }
            Note::Rest {
                duration,
                parameters,
            } => {
                // Combine active params with note's own parameters
                let mut combined_params = active_params.clone(); // Clone here for Rest
                for (k, v) in parameters {
                    combined_params.insert(k.clone(), v.clone());
                }

                let base_duration = *duration;

                // --- Calculate real_duration using correct musical timing logic ---
                let tempo = get_numeric_param(&combined_params, "tempo", 60.0);
                let time_note_val = get_numeric_param(&combined_params, "time_note", 4.0);
                let seconds_per_beat = 60.0 / tempo;
                let beats_per_whole_note = 4.0;
                let beats_per_quarter = 4.0 / time_note_val;
                let whole_note_duration_secs =
                    beats_per_whole_note * beats_per_quarter * seconds_per_beat;
                // +++ Add Debug Logging +++
                debug!(
                    "Rest Calc: base_dur={}, tempo={}, time_note={}, whole_dur_s={}, combined_params={:?}",
                    base_duration,
                    tempo,
                    time_note_val,
                    whole_note_duration_secs,
                    combined_params
                );
                // +++ End Debug Logging +++
                let real_duration = base_duration * whole_note_duration_secs;
                // --- End duration calculation ---

                let instrument_name = get_string_param(&combined_params, "instrument")
                    .unwrap_or_else(|| "basic_synth".to_string());
                producer
                    .yield_(TimedMusicalEvent {
                        time_seconds: start_time,
                        real_duration: real_duration,
                        event: MusicalEventType::Rest {
                            duration_secs: real_duration,
                        },
                        instrument_name,
                    })
                    .await;
                start_time + real_duration
            }
            Note::ParamSetter { key, value } => {
                let instrument_name = get_string_param(active_params, "instrument")
                    .unwrap_or_else(|| "basic_synth".to_string());

                // --- Handle Unset vs Set ---
                match value {
                    ParamValue::Unset => {
                        // Value is Unset, remove the key from active_params
                        debug!("Unsetting parameter: {}", key);
                        active_params.remove(key.as_str());
                        // We still yield a SetParameter event, but with Unset value,
                        // so the synth thread knows the parameter was cleared.
                        producer
                            .yield_(TimedMusicalEvent {
                                time_seconds: start_time,
                                real_duration: 0.0,
                                event: MusicalEventType::SetParameter {
                                    key: key.clone(),
                                    value: value.clone(),
                                }, // Pass ParamValue::Unset
                                instrument_name,
                            })
                            .await;
                    }
                    _ => {
                        // Value is not Unset, perform a normal set/update
                        debug!("Setting parameter: {} = {:?}", key, value);
                        active_params.insert(key.clone(), value.clone());
                        // Yield the standard SetParameter event
                        producer
                            .yield_(TimedMusicalEvent {
                                time_seconds: start_time,
                                real_duration: 0.0,
                                event: MusicalEventType::SetParameter {
                                    key: key.clone(),
                                    value: value.clone(),
                                },
                                instrument_name,
                            })
                            .await;
                    }
                }

                // This arm does not advance time
                start_time
            }
            Note::Envelope(_) => start_time, // Ignore standalone envelopes during generation
            Note::Serial(seq) => {
                let mut current_time = start_time;
                // DO NOT clone active_params here. Pass the mutable reference down directly.
                // Modifications made by ParamSetters within this sequence will affect
                // subsequent items in the same sequence.
                for note_in_seq in seq {
                    if !matches!(note_in_seq, Note::Envelope(_)) {
                        // Pass the original &mut active_params down
                        current_time = Self::generate_events_recursive(
                            note_in_seq,
                            current_time,
                            active_params,
                            producer,
                        )
                        .await;
                    }
                }
                // Modifications to active_params persist after this block for subsequent siblings
                current_time
            }
            Note::Parallel(seq) => {
                // Implementation for Parallel using BinaryHeap
                use std::cmp::Ordering;
                use std::collections::BinaryHeap;

                #[derive(Clone)]
                struct HeapItem {
                    event: TimedMusicalEvent,
                    stream_idx: usize,
                }
                impl Eq for HeapItem {}
                impl PartialEq for HeapItem {
                    fn eq(&self, other: &Self) -> bool {
                        self.event.time_seconds == other.event.time_seconds
                    }
                }
                impl PartialOrd for HeapItem {
                    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                        Some(self.cmp(other))
                    }
                }
                impl Ord for HeapItem {
                    fn cmp(&self, other: &Self) -> Ordering {
                        other.event.time_seconds.total_cmp(&self.event.time_seconds)
                    }
                }

                let mut max_end_time = start_time;

                // Create a function to get event streams
                let event_stream_with_params = |note: &Note,
                                                params: LinkedHashMap<String, ParamValue>,
                                                start_time: f32|
                 -> Box<
                    dyn Iterator<Item = TimedMusicalEvent> + Send + Sync + 'static,
                > {
                    let note_clone = note.clone();
                    Box::new(
                        Gen::new(move |producer: Co<TimedMusicalEvent>| {
                            let mut params_clone = params.clone();
                            async move {
                                let _ = Note::generate_events_recursive(
                                    &note_clone,
                                    start_time,
                                    &mut params_clone,
                                    &producer,
                                )
                                .await;
                            }
                        })
                        .into_iter(),
                    )
                };

                // Give each parallel branch its own INDEPENDENT clone of the active params
                let mut streams: Vec<_> = seq
                    .iter()
                    .filter(|note| !matches!(note, Note::Envelope(_)))
                    // Each stream gets its own clone of the params *at the start* of the par block
                    .map(|note| {
                        let params_clone = active_params.clone();
                        let stream = event_stream_with_params(note, params_clone, start_time);
                        Box::new(stream.peekable())
                            as Box<dyn Iterator<Item = TimedMusicalEvent> + Send + Sync + '_>
                    })
                    .collect();

                let mut heap = BinaryHeap::new();
                for (i, stream) in streams.iter_mut().enumerate() {
                    if let Some(event) = stream.next() {
                        heap.push(HeapItem {
                            event: event.clone(),
                            stream_idx: i,
                        });
                    }
                }

                while let Some(HeapItem { event, stream_idx }) = heap.pop() {
                    max_end_time = max_end_time.max(event.time_seconds + event.real_duration);

                    // *** Important Consideration for Parallel ParamSetters ***
                    // If an event is a SetParameter, it currently *only* affects the heap item generation
                    // logic (which uses event_stream_with_params -> generate_events_recursive).
                    // It does NOT update the `active_params` of the *caller* of this parallel block,
                    // nor does it affect other parallel branches directly after being yielded.
                    // This is generally the desired behavior for `par`.
                    producer.yield_(event).await;

                    if let Some(next_event) = streams[stream_idx].next() {
                        heap.push(HeapItem {
                            event: next_event.clone(),
                            stream_idx,
                        });
                    }
                }
                // Updates to params within parallel branches are discarded.
                max_end_time
            }
            Note::ParallelMin(seq) => {
                // Similar to Parallel, but returns the MINIMUM end time
                use std::cmp::Ordering;
                use std::collections::BinaryHeap;

                #[derive(Clone)]
                struct HeapItemPM {
                    event: TimedMusicalEvent,
                    stream_idx: usize,
                } // Use different name to avoid scope conflict
                impl Eq for HeapItemPM {}
                impl PartialEq for HeapItemPM {
                    fn eq(&self, other: &Self) -> bool {
                        self.event.time_seconds == other.event.time_seconds
                    }
                }
                impl PartialOrd for HeapItemPM {
                    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                        Some(self.cmp(other))
                    }
                }
                impl Ord for HeapItemPM {
                    fn cmp(&self, other: &Self) -> Ordering {
                        other.event.time_seconds.total_cmp(&self.event.time_seconds)
                    }
                }

                let mut branch_end_times = if matches!(note, Note::ParallelMin(_)) {
                    Some(Vec::new())
                } else {
                    None
                }; // Only needed for ParallelMin

                // Create a function to get event streams
                let event_stream_with_params = |note: &Note,
                                                params: LinkedHashMap<String, ParamValue>,
                                                start_time: f32|
                 -> Box<
                    dyn Iterator<Item = TimedMusicalEvent> + Send + Sync + 'static,
                > {
                    let note_clone = note.clone();
                    Box::new(
                        Gen::new(move |producer: Co<TimedMusicalEvent>| {
                            let mut params_clone = params.clone();
                            async move {
                                let _ = Note::generate_events_recursive(
                                    &note_clone,
                                    start_time,
                                    &mut params_clone,
                                    &producer,
                                )
                                .await;
                            }
                        })
                        .into_iter(),
                    )
                };

                let mut streams: Vec<_> = seq
                    .iter()
                    .filter(|note| !matches!(note, Note::Envelope(_)))
                    // Pass only active_params clone to event_stream_with_params
                    .map(|note| {
                        let params_clone = active_params.clone();
                        let stream = event_stream_with_params(note, params_clone, start_time);
                        Box::new(stream.peekable())
                            as Box<dyn Iterator<Item = TimedMusicalEvent> + Send + Sync + '_>
                    })
                    .collect();

                let mut heap = BinaryHeap::new();
                for (i, stream) in streams.iter_mut().enumerate() {
                    if let Some(event) = stream.next() {
                        heap.push(HeapItemPM {
                            event: event.clone(),
                            stream_idx: i,
                        });
                        // Capture the potential end time from the first event
                        if let Some(ref mut times) = branch_end_times {
                            let initial_end = event.time_seconds + event.real_duration;
                            times.push(initial_end);
                        }
                    } else {
                        // Handle empty branches - they contribute start_time to min calculation
                        if let Some(ref mut times) = branch_end_times {
                            times.push(start_time);
                        }
                    }
                }

                // Find initial minimum end time from first events
                let min_end_time = branch_end_times // Calculate and assign directly
                    .map(|times| times.iter().fold(f32::MAX, |a, &b| a.min(b)))
                    .unwrap_or(f32::MAX); // Default value if not ParallelMin

                while let Some(HeapItemPM { event, stream_idx }) = heap.pop() {
                    // Yield event only if it starts before the minimum end time
                    if event.time_seconds < min_end_time {
                        producer.yield_(event).await;
                    } else {
                        // Don't yield events that start at or after the determined minimum duration
                        // We still need to process the stream to potentially find earlier events though
                    }

                    // Get next event from the same stream
                    if let Some(next_event) = streams[stream_idx].next() {
                        // Only push if the next event *starts* before the min_end_time
                        // This prevents infinite loops if a branch has events past the cutoff
                        if next_event.time_seconds < min_end_time {
                            heap.push(HeapItemPM {
                                event: next_event.clone(),
                                stream_idx,
                            });
                        }
                    }
                }
                // The overall end time for parmin is the minimum of the individual branch end times
                min_end_time
            }
            Note::ForkSequence(seq) => {
                let mut current_time = start_time;
                // Use a clone of params so modifications don't leak out
                let mut fork_params = active_params.clone();
                for note_in_seq in seq {
                    if !matches!(note_in_seq, Note::Envelope(_)) {
                        // Pass the fork's own param map down
                        current_time = Self::generate_events_recursive(
                            note_in_seq,
                            current_time,
                            &mut fork_params,
                            producer,
                        )
                        .await;
                    }
                }
                start_time // Return the original start time, ignoring the sequence duration
            }
            Note::ForkParallel(seq) => {
                // Same logic as Parallel for event generation
                use std::cmp::Ordering;
                use std::collections::BinaryHeap;

                #[derive(Clone)]
                struct HeapItemFP {
                    event: TimedMusicalEvent,
                    stream_idx: usize,
                }
                impl Eq for HeapItemFP {}
                impl PartialEq for HeapItemFP {
                    fn eq(&self, other: &Self) -> bool {
                        self.event.time_seconds == other.event.time_seconds
                    }
                }
                impl PartialOrd for HeapItemFP {
                    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                        Some(self.cmp(other))
                    }
                }
                impl Ord for HeapItemFP {
                    fn cmp(&self, other: &Self) -> Ordering {
                        other.event.time_seconds.total_cmp(&self.event.time_seconds)
                    }
                }

                // Create a function to get event streams
                let event_stream_with_params = |note: &Note,
                                                params: LinkedHashMap<String, ParamValue>,
                                                start_time: f32|
                 -> Box<
                    dyn Iterator<Item = TimedMusicalEvent> + Send + Sync + 'static,
                > {
                    let note_clone = note.clone();
                    Box::new(
                        Gen::new(move |producer: Co<TimedMusicalEvent>| {
                            let mut params_clone = params.clone();
                            async move {
                                let _ = Note::generate_events_recursive(
                                    &note_clone,
                                    start_time,
                                    &mut params_clone,
                                    &producer,
                                )
                                .await;
                            }
                        })
                        .into_iter(),
                    )
                };

                let mut streams: Vec<_> = seq
                    .iter()
                    .filter(|note| !matches!(note, Note::Envelope(_)))
                    // Each branch gets its own clone, like regular Parallel
                    .map(|note| {
                        let params_clone = active_params.clone();
                        let stream = event_stream_with_params(note, params_clone, start_time);
                        Box::new(stream.peekable())
                            as Box<dyn Iterator<Item = TimedMusicalEvent> + Send + Sync + '_>
                    })
                    .collect();

                let mut heap = BinaryHeap::new();
                for (i, stream) in streams.iter_mut().enumerate() {
                    if let Some(event) = stream.next() {
                        heap.push(HeapItemFP {
                            event: event.clone(),
                            stream_idx: i,
                        });
                    }
                }

                while let Some(HeapItemFP { event, stream_idx }) = heap.pop() {
                    producer.yield_(event).await;
                    if let Some(next_event) = streams[stream_idx].next() {
                        heap.push(HeapItemFP {
                            event: next_event.clone(),
                            stream_idx,
                        });
                    }
                }
                // Param updates are discarded, like regular Parallel
                start_time // Return the original start time, ignoring the parallel block duration
            }
            Note::Comment(comment_str) => {
                let instrument_name = get_string_param(active_params, "instrument")
                    .unwrap_or_else(|| "basic_synth".to_string());
                producer
                    .yield_(TimedMusicalEvent {
                        time_seconds: start_time,
                        real_duration: 0.0, // Comments take no musical time
                        event: MusicalEventType::Comment(comment_str.clone()),
                        instrument_name,
                    })
                    .await;
                start_time // Does not advance time
            }
            // These should have been resolved by ser/par processing
            Note::AtomImplicitDuration { .. }
            | Note::PreviousPitch { .. }
            | Note::DurationTie { .. }
            | Note::RepeatMarker(_) => {
                panic!(
                    "Internal Error: Unresolved Note variant found during event generation: {:?}",
                    note
                );
            }
        }
    }

    /// Returns an iterator over timed musical events generated from this Note.
    pub fn event_stream(
        &self,
        start_time: f32,
    ) -> impl Iterator<Item = TimedMusicalEvent> + Send + Sync + 'static {
        let note = self.clone();
        Gen::new(move |producer: Co<TimedMusicalEvent>| async move {
            let mut initial_params: LinkedHashMap<String, ParamValue> = LinkedHashMap::new();
            let _ =
                Note::generate_events_recursive(&note, start_time, &mut initial_params, &producer)
                    .await;
        })
        .into_iter()
    }

    pub fn event_stream_with_params(
        &self,
        params: LinkedHashMap<String, ParamValue>,
        start_time: f32,
    ) -> impl Iterator<Item = TimedMusicalEvent> + Send + Sync + 'static {
        let note = self.clone();
        // Assume macros are managed by the caller (like note_stream_to_event_stream)
        // or start empty if called directly. For simplicity here, start empty.
        Gen::new(move |producer: Co<TimedMusicalEvent>| async move {
            let mut active_params = params.clone();
            let _ =
                Note::generate_events_recursive(&note, start_time, &mut active_params, &producer)
                    .await;
        })
        .into_iter()
    }
}

/// Iterator over Notes
pub type NoteIterator = Box<dyn Iterator<Item = Note> + Send + Sync + 'static>;

/// Iterator over TimedMusicalEvents
pub type TimedMusicalEventIterator =
    Box<dyn Iterator<Item = TimedMusicalEvent> + Send + Sync + 'static>;

/// Type alias for the shared REPL context (event queue Arc and synth time Arc)
pub type ReplContext = Arc<(Arc<Mutex<VecDeque<TimedMusicalEvent>>>, Arc<Mutex<f32>>)>;

/// Processes a sequence, resolving implicit durations/pitches, handling ties and repeats.
pub fn ser<I>(items: I) -> Note
where
    I: IntoIterator<Item = Note>,
{
    let mut resolved_items: Vec<Note> = Vec::new();
    let mut current_attrs: LinkedHashMap<String, ParamValue> = LinkedHashMap::new();
    let mut last_duration: Option<f32> = None;
    let mut last_pitch_midi: Option<u8> = None;

    for item in items {
        // --- Special Handling for Markers ---
        match &item {
            Note::DurationTie {
                duration: tie_duration,
            } => {
                match resolved_items.last_mut() {
                    Some(Note::Atom { duration, midi, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                        last_pitch_midi = Some(*midi);
                    }
                    Some(Note::Rest { duration, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                    }
                    _ => panic!("DurationTie must follow a note or rest."),
                }
                continue; // Skip rest of loop for Tie
            }
            Note::RepeatMarker(count) => {
                let repeat_count = *count;
                if repeat_count == 0 {
                    continue; // Repeating 0 times does nothing
                }
                let last_resolved = resolved_items
                    .pop()
                    .expect("repeat!() must follow a note, rest, ser(), or par() block.");

                // Check if the item is repeatable
                match &last_resolved {
                    Note::Atom { .. } | Note::Rest { .. } | Note::Serial(_) | Note::Parallel(_) => {
                    }
                    _ => panic!("Cannot repeat ParamSetter, Envelope, Tie, or RepeatMarker."),
                }

                // Push the original back first!
                resolved_items.push(last_resolved.clone());
                last_duration = last_resolved.get_base_duration().or(last_duration);
                if let Note::Atom { midi, .. } = last_resolved {
                    last_pitch_midi = Some(midi);
                }

                // Now repeat N times
                for _ in 0..repeat_count {
                    let item_to_repeat = last_resolved.clone();

                    // Resolve implicit parts (based on context *before* this repetition)
                    let current_item_resolved = match &item_to_repeat {
                        Note::AtomImplicitDuration { midi, parameters } => {
                            let duration = last_duration
                                .expect("Repeat: Implicit duration needs preceding note/rest.");
                            Note::Atom {
                                midi: *midi,
                                duration,
                                parameters: parameters.clone(),
                            }
                        }
                        Note::PreviousPitch {
                            optional_duration,
                            parameters,
                        } => {
                            let pitch = last_pitch_midi.unwrap_or(60);
                            let duration = optional_duration
                                .or(last_duration)
                                .expect("Repeat: 'p' note needs preceding note/rest.");
                            Note::Atom {
                                midi: pitch,
                                duration,
                                parameters: parameters.clone(),
                            }
                        }
                        _ => item_to_repeat,
                    };

                    // Apply current attributes
                    let item_with_attrs = match &current_item_resolved {
                        Note::ParamSetter { .. }
                        | Note::Envelope { .. }
                        | Note::DurationTie { .. }
                        | Note::RepeatMarker(_) => current_item_resolved.clone(),
                        _ => apply_parameters(current_item_resolved.clone(), &current_attrs),
                    };

                    // Update context based on the *repeated* item and push
                    match &item_with_attrs {
                        Note::ParamSetter { key, value } => {
                            current_attrs.insert(key.clone(), value.clone());
                            resolved_items.push(item_with_attrs.clone());
                        }
                        note @ Note::Atom { midi, duration, .. } => {
                            last_duration = Some(*duration);
                            last_pitch_midi = Some(*midi);
                            resolved_items.push(note.clone());
                        }
                        note @ Note::Rest { duration, .. } => {
                            last_duration = Some(*duration);
                            resolved_items.push(note.clone());
                        }
                        _ => resolved_items.push(item_with_attrs.clone()),
                    }
                }
                continue; // Skip rest of loop for Repeat
            }
            _ => { /* Not a Tie or Repeat marker, proceed normally */ }
        }

        // --- Normal Processing (moved from previous structure) ---

        // 2. Resolve implicit duration and/or pitch (item is already cloned if needed)
        let current_item_resolved = match &item {
            Note::AtomImplicitDuration { midi, parameters } => {
                let duration = last_duration.expect(
                    "Cannot use implicit duration note before a note/rest with explicit duration.",
                );
                Note::Atom {
                    midi: *midi,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            Note::PreviousPitch {
                optional_duration,
                parameters,
            } => {
                let pitch = last_pitch_midi.unwrap_or(60);
                let duration = optional_duration.or(last_duration).expect("Cannot use 'p' note (PreviousPitch) before a note/rest with explicit duration.");
                Note::Atom {
                    midi: pitch,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            _ => item.clone(), // Clone Atom, Rest, Serial, Parallel, Envelope, AttrSetter
        };

        // 3. Apply current attributes
        let item_with_attrs = match &current_item_resolved {
            Note::Envelope(_)
            | Note::ParamSetter { .. }
            | Note::DurationTie { .. }
            | Note::RepeatMarker(_) => current_item_resolved.clone(),
            _ => apply_parameters(current_item_resolved.clone(), &current_attrs),
        };

        // 4. Handle Envelopes, ParamSetters, update context for Atoms/Rests
        match &item_with_attrs {
            Note::Envelope(_) => {
                panic!("Bare env!() macro usage is not allowed; use param!(key=env!(...)) instead");
            }
            Note::ParamSetter { key, value } => {
                current_attrs.insert(key.clone(), value.clone());
                resolved_items.push(item_with_attrs.clone());
            }
            note @ Note::Atom { midi, duration, .. } => {
                last_duration = Some(*duration);
                last_pitch_midi = Some(*midi);
                resolved_items.push(note.clone());
            }
            note @ Note::Rest { duration, .. } => {
                last_duration = Some(*duration);
                resolved_items.push(note.clone());
            }
            // Serial/Parallel pushed without updating context
            // Implicit/Previous/Tie/Repeat handled earlier
            _ => {
                resolved_items.push(item_with_attrs.clone());
            }
        }
    }
    Note::Serial(resolved_items)
}

/// Processes parallel notes. Implicit durations/pitches inherit; ties apply.
pub fn par<I>(items: I) -> Note
where
    I: IntoIterator<Item = Note>,
{
    let mut vec_items = Vec::new();
    let mut current_attrs: LinkedHashMap<String, ParamValue> = LinkedHashMap::new();
    // Bare env!() macros are not allowed in par(); use param!(key=env!(...)) instead
    let mut last_duration: Option<f32> = None; // Tracks last *base* duration
    let mut last_pitch_midi: Option<u8> = None; // Tracks last pitch (default C4=60)
    let mut last_note_index: Option<usize> = None; // Tracks index for ties

    for item in items {
        // 1. Handle DurationTie first
        if let Note::DurationTie {
            duration: tie_duration,
        } = &item
        {
            if let Some(idx) = last_note_index {
                match vec_items.get_mut(idx) {
                    Some(Note::Atom { duration, midi, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                        last_pitch_midi = Some(*midi);
                    }
                    Some(Note::Rest { duration, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                    }
                    _ => {}
                }
            } else {
                panic!("DurationTie inside par() must follow a note/rest within the same par().");
            }
            continue;
        }

        // 2. Resolve implicit duration and/or pitch
        let current_item_resolved = match &item {
            Note::AtomImplicitDuration { midi, parameters } => {
                let duration = last_duration
                    .expect("Implicit duration note in par() requires preceding note/rest.");
                Note::Atom {
                    midi: *midi,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            Note::PreviousPitch {
                optional_duration,
                parameters,
            } => {
                let pitch = last_pitch_midi.unwrap_or(60);
                let duration = optional_duration
                    .or(last_duration)
                    .expect("'p' note in par() requires preceding note/rest.");
                Note::Atom {
                    midi: pitch,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            _ => item.clone(),
        };

        // 3. Apply current attributes
        let item_with_attrs = match &current_item_resolved {
            Note::Envelope(_) | Note::ParamSetter { .. } | Note::DurationTie { .. } => {
                current_item_resolved.clone()
            }
            _ => apply_parameters(current_item_resolved.clone(), &current_attrs),
        };

        // 4. Handle items: Collect Envs/Attrs, Push notes/rests, Update context
        match &item_with_attrs {
            Note::Envelope(_) => {
                panic!("Bare env!() macro usage is not allowed; use param!(key=env!(...)) instead");
            }
            Note::ParamSetter { key, value } => {
                current_attrs.insert(key.clone(), value.clone());
                // ParamSetters within par() might update context but aren't added to vec_items
            }
            note @ Note::Atom { midi, duration, .. } => {
                last_duration = Some(*duration);
                last_pitch_midi = Some(*midi);
                vec_items.push(note.clone());
                last_note_index = Some(vec_items.len() - 1);
            }
            note @ Note::Rest { duration, .. } => {
                last_duration = Some(*duration);
                vec_items.push(note.clone());
                last_note_index = Some(vec_items.len() - 1);
            }
            // Serial/Parallel pushed without updating context
            // Implicit/Previous handled earlier, Tie handled earlier
            _ => {
                vec_items.push(item_with_attrs.clone());
                last_note_index = None;
            }
        }
    }

    // Return the parallel note sequence
    Note::Parallel(vec_items)
}

/// Processes parallel notes, duration determined by the SHORTEST note.
pub fn parmin<I>(items: I) -> Note
where
    I: IntoIterator<Item = Note>,
{
    let mut vec_items = Vec::new();
    let mut current_attrs: LinkedHashMap<String, ParamValue> = LinkedHashMap::new();
    let mut last_duration: Option<f32> = None;
    let mut last_pitch_midi: Option<u8> = None;
    let mut last_note_index: Option<usize> = None;

    for item in items {
        if let Note::DurationTie {
            duration: tie_duration,
        } = &item
        {
            if let Some(idx) = last_note_index {
                match vec_items.get_mut(idx) {
                    Some(Note::Atom { duration, midi, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                        last_pitch_midi = Some(*midi);
                    }
                    Some(Note::Rest { duration, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                    }
                    _ => {}
                }
            } else {
                panic!(
                    "DurationTie inside parmin() must follow a note/rest within the same parmin()."
                );
            }
            continue;
        }

        let current_item_resolved = match &item {
            Note::AtomImplicitDuration { midi, parameters } => {
                let duration = last_duration
                    .expect("Implicit duration note in parmin() requires preceding note/rest.");
                Note::Atom {
                    midi: *midi,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            Note::PreviousPitch {
                optional_duration,
                parameters,
            } => {
                let pitch = last_pitch_midi.unwrap_or(60);
                let duration = optional_duration
                    .or(last_duration)
                    .expect("'p' note in parmin() requires preceding note/rest.");
                Note::Atom {
                    midi: pitch,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            _ => item.clone(),
        };

        let item_with_attrs = match &current_item_resolved {
            Note::Envelope(_) | Note::ParamSetter { .. } | Note::DurationTie { .. } => {
                current_item_resolved.clone()
            }
            _ => apply_parameters(current_item_resolved.clone(), &current_attrs),
        };

        match &item_with_attrs {
            Note::Envelope(_) => {
                panic!("Bare env!() macro usage is not allowed; use param!(key=env!(...)) instead");
            }
            Note::ParamSetter { key, value } => {
                current_attrs.insert(key.clone(), value.clone());
            }
            note @ Note::Atom { midi, duration, .. } => {
                last_duration = Some(*duration);
                last_pitch_midi = Some(*midi);
                vec_items.push(note.clone());
                last_note_index = Some(vec_items.len() - 1);
            }
            note @ Note::Rest { duration, .. } => {
                last_duration = Some(*duration);
                vec_items.push(note.clone());
                last_note_index = Some(vec_items.len() - 1);
            }
            _ => {
                vec_items.push(item_with_attrs.clone());
                last_note_index = None;
            }
        }
    }

    Note::ParallelMin(vec_items)
}

/// Processes a sequence without advancing the outer time cursor.
pub fn forkseq<I>(items: I) -> Note
where
    I: IntoIterator<Item = Note>,
{
    // Essentially the same logic as ser(), but returns ForkSequence
    let mut resolved_items: Vec<Note> = Vec::new();
    let mut current_attrs: LinkedHashMap<String, ParamValue> = LinkedHashMap::new();
    let mut last_duration: Option<f32> = None;
    let mut last_pitch_midi: Option<u8> = None;

    for item in items {
        match &item {
            Note::DurationTie {
                duration: tie_duration,
            } => {
                match resolved_items.last_mut() {
                    Some(Note::Atom { duration, midi, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                        last_pitch_midi = Some(*midi);
                    }
                    Some(Note::Rest { duration, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                    }
                    _ => panic!("DurationTie must follow a note or rest."),
                }
                continue;
            }
            Note::RepeatMarker(count) => {
                // Copy repeat logic from ser()
                let repeat_count = *count;
                if repeat_count == 0 {
                    continue;
                }
                let last_resolved = resolved_items
                    .pop()
                    .expect("repeat!() must follow a note, rest, ser(), or par() block.");
                match &last_resolved {
                    Note::Atom { .. }
                    | Note::Rest { .. }
                    | Note::Serial(_)
                    | Note::Parallel(_)
                    | Note::ForkSequence(_)
                    | Note::ForkParallel(_)
                    | Note::ParallelMin(_) => {}
                    _ => panic!("Cannot repeat ParamSetter, Envelope, Tie, or RepeatMarker."),
                }
                resolved_items.push(last_resolved.clone());
                last_duration = last_resolved.get_base_duration().or(last_duration);
                if let Note::Atom { midi, .. } = last_resolved {
                    last_pitch_midi = Some(midi);
                }
                for _ in 0..repeat_count {
                    let item_to_repeat = last_resolved.clone();
                    let current_item_resolved = match &item_to_repeat {
                        Note::AtomImplicitDuration { midi, parameters } => {
                            let duration = last_duration
                                .expect("Repeat: Implicit duration needs preceding note/rest.");
                            Note::Atom {
                                midi: *midi,
                                duration,
                                parameters: parameters.clone(),
                            }
                        }
                        Note::PreviousPitch {
                            optional_duration,
                            parameters,
                        } => {
                            let pitch = last_pitch_midi.unwrap_or(60);
                            let duration = optional_duration
                                .or(last_duration)
                                .expect("Repeat: 'p' note needs preceding note/rest.");
                            Note::Atom {
                                midi: pitch,
                                duration,
                                parameters: parameters.clone(),
                            }
                        }
                        _ => item_to_repeat,
                    };
                    let item_with_attrs = match &current_item_resolved {
                        Note::ParamSetter { .. }
                        | Note::Envelope { .. }
                        | Note::DurationTie { .. }
                        | Note::RepeatMarker(_) => current_item_resolved.clone(),
                        _ => apply_parameters(current_item_resolved.clone(), &current_attrs),
                    };
                    match &item_with_attrs {
                        Note::ParamSetter { key, value } => {
                            current_attrs.insert(key.clone(), value.clone());
                            resolved_items.push(item_with_attrs.clone());
                        }
                        note @ Note::Atom { midi, duration, .. } => {
                            last_duration = Some(*duration);
                            last_pitch_midi = Some(*midi);
                            resolved_items.push(note.clone());
                        }
                        note @ Note::Rest { duration, .. } => {
                            last_duration = Some(*duration);
                            resolved_items.push(note.clone());
                        }
                        _ => resolved_items.push(item_with_attrs.clone()),
                    }
                }
                continue;
            }
            _ => { /* Normal processing */ }
        }

        let current_item_resolved = match &item {
            Note::AtomImplicitDuration { midi, parameters } => {
                let duration = last_duration
                    .expect("Implicit duration note in forkseq() requires preceding note/rest.");
                Note::Atom {
                    midi: *midi,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            Note::PreviousPitch {
                optional_duration,
                parameters,
            } => {
                let pitch = last_pitch_midi.unwrap_or(60);
                let duration = optional_duration
                    .or(last_duration)
                    .expect("'p' note in forkseq() requires preceding note/rest.");
                Note::Atom {
                    midi: pitch,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            _ => item.clone(),
        };

        let item_with_attrs = match &current_item_resolved {
            Note::Envelope(_)
            | Note::ParamSetter { .. }
            | Note::DurationTie { .. }
            | Note::RepeatMarker(_) => current_item_resolved.clone(),
            _ => apply_parameters(current_item_resolved.clone(), &current_attrs),
        };

        match &item_with_attrs {
            Note::Envelope(_) => {
                panic!("Bare env!() macro usage is not allowed; use param!(key=env!(...)) instead")
            }
            Note::ParamSetter { key, value } => {
                current_attrs.insert(key.clone(), value.clone());
                resolved_items.push(item_with_attrs.clone());
            }
            note @ Note::Atom { midi, duration, .. } => {
                last_duration = Some(*duration);
                last_pitch_midi = Some(*midi);
                resolved_items.push(note.clone());
            }
            note @ Note::Rest { duration, .. } => {
                last_duration = Some(*duration);
                resolved_items.push(note.clone());
            }
            _ => resolved_items.push(item_with_attrs.clone()),
        }
    }
    Note::ForkSequence(resolved_items)
}

/// Processes parallel notes without advancing the outer time cursor.
pub fn forkpar<I>(items: I) -> Note
where
    I: IntoIterator<Item = Note>,
{
    // Essentially the same logic as par(), but returns ForkParallel
    let mut vec_items = Vec::new();
    let mut current_attrs: LinkedHashMap<String, ParamValue> = LinkedHashMap::new();
    let mut last_duration: Option<f32> = None;
    let mut last_pitch_midi: Option<u8> = None;
    let mut last_note_index: Option<usize> = None;

    for item in items {
        if let Note::DurationTie {
            duration: tie_duration,
        } = &item
        {
            if let Some(idx) = last_note_index {
                match vec_items.get_mut(idx) {
                    Some(Note::Atom { duration, midi, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                        last_pitch_midi = Some(*midi);
                    }
                    Some(Note::Rest { duration, .. }) => {
                        *duration += tie_duration;
                        last_duration = Some(*duration);
                    }
                    _ => {}
                }
            } else {
                panic!("DurationTie inside forkpar() must follow a note/rest within the same forkpar().");
            }
            continue;
        }

        let current_item_resolved = match &item {
            Note::AtomImplicitDuration { midi, parameters } => {
                let duration = last_duration
                    .expect("Implicit duration note in forkpar() requires preceding note/rest.");
                Note::Atom {
                    midi: *midi,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            Note::PreviousPitch {
                optional_duration,
                parameters,
            } => {
                let pitch = last_pitch_midi.unwrap_or(60);
                let duration = optional_duration
                    .or(last_duration)
                    .expect("'p' note in forkpar() requires preceding note/rest.");
                Note::Atom {
                    midi: pitch,
                    duration,
                    parameters: parameters.clone(),
                }
            }
            _ => item.clone(),
        };

        let item_with_attrs = match &current_item_resolved {
            Note::Envelope(_) | Note::ParamSetter { .. } | Note::DurationTie { .. } => {
                current_item_resolved.clone()
            }
            _ => apply_parameters(current_item_resolved.clone(), &current_attrs),
        };

        match &item_with_attrs {
            Note::Envelope(_) => {
                panic!("Bare env!() macro usage is not allowed; use param!(key=env!(...)) instead");
            }
            Note::ParamSetter { key, value } => {
                current_attrs.insert(key.clone(), value.clone());
            }
            note @ Note::Atom { midi, duration, .. } => {
                last_duration = Some(*duration);
                last_pitch_midi = Some(*midi);
                vec_items.push(note.clone());
                last_note_index = Some(vec_items.len() - 1);
            }
            note @ Note::Rest { duration, .. } => {
                last_duration = Some(*duration);
                vec_items.push(note.clone());
                last_note_index = Some(vec_items.len() - 1);
            }
            _ => {
                vec_items.push(item_with_attrs.clone());
                last_note_index = None;
            }
        }
    }

    Note::ForkParallel(vec_items)
}

fn apply_parameters(mut note: Note, params: &LinkedHashMap<String, ParamValue>) -> Note {
    match &mut note {
        Note::Atom { parameters, .. }
        | Note::Rest { parameters, .. }
        | Note::AtomImplicitDuration { parameters, .. }
        | Note::PreviousPitch { parameters, .. } => { // Combined arm for variants with parameters
            // Iterate through the input params and update/insert into the note's Vec
            for (key_to_apply, value_to_apply) in params.iter() {
                 if key_to_apply == "key" { continue; } // Skip applying 'key'

                 // Find if the key already exists in the note's parameters
                 if let Some(existing_param) = parameters.iter_mut().find(|(k, _)| k == key_to_apply) {
                     // Update existing parameter
                     existing_param.1 = value_to_apply.clone();
                 } else {
                     // Insert new parameter
                     parameters.push((key_to_apply.clone(), value_to_apply.clone()));
                 }
             }
        },
        Note::DurationTie { .. }
        | Note::RepeatMarker(_)
        | Note::ParamSetter { .. }
        | Note::Envelope(_)
        | Note::Comment(_) // Comments don't receive parameters
        => {
            // These variants do not have parameters applied to them directly
            /* No-op */
        },
        Note::Serial(notes) | Note::Parallel(notes) => {
            for n in notes.iter_mut() {
                *n = apply_parameters(n.clone(), params);
            }
        },
        Note::ParallelMin(notes) | Note::ForkSequence(notes) | Note::ForkParallel(notes) => {
             for n in notes.iter_mut() {
                *n = apply_parameters(n.clone(), params);
            }
        },
    }
    note
}

/// Convert an iterator of notes to a flattened event stream iterator
///
/// This iterator takes a stream of notes and produces a flattened stream of events.
/// It handles sequential processing, maintaining timing and parameter context across notes.
///
/// The input stream is a stream of notes.
/// The output stream is a stream of events.
pub fn note_stream_to_event_stream(
    note_stream: NoteIterator,
    start_time: f32,
) -> TimedMusicalEventIterator {
    let generator = Gen::new(|producer: Co<TimedMusicalEvent>| async move {
        let mut current_time = start_time; // <-- Use provided start_time
                                           // Start with default parameters, they might be updated by ParamSetter events
        let mut active_params = LinkedHashMap::new();

        for note in note_stream {
            // Generate events for the current note
            let mut note_params_clone = active_params.clone();

            // Use a sub-generator to get events for this note
            let sub_gen = Gen::new(|sub_producer: Co<TimedMusicalEvent>| async move {
                let _ = Note::generate_events_recursive(
                    &note,
                    current_time, // Pass down the current (potentially advanced) time
                    &mut note_params_clone,
                    &sub_producer,
                )
                .await;
            });

            let mut note_events_iter = sub_gen.into_iter();
            let mut max_end_time_for_this_note = current_time;

            // Process all events generated by this single note
            while let Some(event) = note_events_iter.next() {
                max_end_time_for_this_note =
                    max_end_time_for_this_note.max(event.time_seconds + event.real_duration);

                // If the event is a parameter setter, update the *top-level* active parameters
                if let MusicalEventType::SetParameter { key, value } = &event.event {
                    // ParamSetters yielded from the note's stream update the main context
                    active_params.insert(key.clone(), value.clone());
                }

                // Yield the event
                producer.yield_(event).await;
            }

            // Update the global current time
            current_time = max_end_time_for_this_note;
        }
    });

    Box::new(generator.into_iter())
}

// Transpose Note Logic
pub fn transpose_note(note: &Note, offset: i8) -> Note {
    if offset == 0 {
        return note.clone();
    } // No-op if offset is zero

    match note {
        Note::Atom {
            midi,
            duration,
            parameters,
        } => {
            let new_midi = (*midi as i16 + offset as i16).clamp(0, 127) as u8;
            Note::Atom {
                midi: new_midi,
                duration: *duration,
                parameters: parameters.clone(),
            }
        }
        Note::AtomImplicitDuration { midi, parameters } => {
            let new_midi = (*midi as i16 + offset as i16).clamp(0, 127) as u8;
            Note::AtomImplicitDuration {
                midi: new_midi,
                parameters: parameters.clone(),
            }
        }
        Note::PreviousPitch {
            optional_duration: _,
            parameters: _,
        } => {
            // Ignore transpose for PreviousPitch to avoid complex state lookbehind
            warn!("Transpose applied to PreviousPitch ('p') note is ignored.");
            note.clone()
        }
        Note::Serial(notes) => {
            Note::Serial(notes.iter().map(|n| transpose_note(n, offset)).collect())
        }
        Note::Parallel(notes) => {
            Note::Parallel(notes.iter().map(|n| transpose_note(n, offset)).collect())
        }
        Note::ParallelMin(notes) => {
            Note::ParallelMin(notes.iter().map(|n| transpose_note(n, offset)).collect())
        }
        Note::ForkSequence(notes) => {
            Note::ForkSequence(notes.iter().map(|n| transpose_note(n, offset)).collect())
        }
        Note::ForkParallel(notes) => {
            Note::ForkParallel(notes.iter().map(|n| transpose_note(n, offset)).collect())
        }
        // Non-pitch related notes are returned unchanged
        Note::Rest { .. }
        | Note::ParamSetter { .. }
        | Note::Envelope(_)
        | Note::DurationTie { .. }
        | Note::RepeatMarker(_)
        | Note::Comment(_) => note.clone(),
    }
}

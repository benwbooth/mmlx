extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, LitFloat, LitInt};

// --- Shared Definitions ---

const DURATIONS: [(&str, f32); 8] = [
    ("w", 1.0f32),
    ("h", 0.5),
    ("q", 0.25),
    ("e", 0.125),
    ("i", 0.0625),
    ("t", 0.03125),
    ("x", 0.015625),
    ("o", 0.0078125),
];

const PITCHES: [(&str, i8); 7] = [
    ("c", 0),
    ("d", 2),
    ("e", 4),
    ("f", 5),
    ("g", 7),
    ("a", 9),
    ("b", 11),
];

// Note: acc_val adjusted to i8 to match midi calculation later
const ACCIDENTALS: [(&str, i8); 7] = [
    ("", 0),
    ("s", 1),
    ("f", -1),
    ("ss", 2),
    ("ff", -2),
    ("n", 0),
    ("nn", 0),
];

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

/// Musical value words: ("quarter note (1/4)"), dots 0-3 add dotting.
/// Human-readable pitch: "middle C (C4), MIDI 60".
fn pitch_words(letter: &str, acc_sym: &str, octave: i32, midi: i32) -> String {
    let spelled = format!("{}{}{}", letter.to_uppercase(), acc_sym, octave);
    if letter == "c" && acc_sym.is_empty() && octave == 4 {
        format!("middle C ({spelled}), MIDI {midi}")
    } else {
        format!("{spelled}, MIDI {midi}")
    }
}

fn value_words(code: &str, dots: u8) -> String {
    let (name, num, den) = match code {
        "w" => ("whole", 1, 1),
        "h" => ("half", 1, 2),
        "q" => ("quarter", 1, 4),
        "e" => ("eighth", 1, 8),
        "i" => ("sixteenth", 1, 16),
        "t" => ("thirty-second", 1, 32),
        "x" => ("sixty-fourth", 1, 64),
        _ => ("hundred-twenty-eighth", 1, 128),
    };
    let (dotted, mult_num, mult_den) = match dots {
        1 => ("dotted ", 3, 2),
        2 => ("double-dotted ", 7, 4),
        3 => ("triple-dotted ", 15, 8),
        _ => ("", 1, 1),
    };
    let (mut num, mut den) = (num * mult_num, den * mult_den);
    let g = gcd(num, den);
    num /= g;
    den /= g;
    format!("{dotted}{name} note ({num}/{den})")
}

// --- Macro 1: Generate Rest Notes (rw, rhd, etc.) ---
#[proc_macro]
pub fn generate_rest_notes(_input: TokenStream) -> TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();
    for &(code, base_dur) in &DURATIONS {
        // Function to generate the macro_rules! definition
        let generate_macro = |ident: &Ident,
                              const_ident: &Ident,
                              doc: String|
         -> proc_macro2::TokenStream {
            let name = ident.to_string();
            let macrodock = format!(
                "{doc}\n\nMacro form — attach per-note attributes, e.g. `{name}!(velocity = 90)`."
            );
            quote! {
                #[macro_export]
                #[doc = #macrodock]
                macro_rules! #ident {
                    () => { $crate::play::note::#const_ident };
                    // --- Updated pattern to allow optional value ---
                    ($($key:ident = $($value:expr)? ),* $(,)?) => {{
                        let mut note = $crate::play::note::#const_ident.clone();
                        $(
                            // Check if value is present or not
                            let setter = if let Some(val_expr) = maybe_value { // <<< Problem: How to capture optional value?
                                // Standard key = value case
                                $crate::play::create_param_setter(stringify!($key), val_expr)
                            } else {
                                // key= case (unset)
                                $crate::play::create_param_setter(stringify!($key), $crate::play::ParamValue::Unset)
                            };

                            // --- Need a different approach for macro_rules! parsing ---
                            // Let's try parsing each individually inside the loop.
                            // Option 1: Use a sub-macro (TT muncher style) - complex
                            // Option 2: Pass token tree to helper function - requires helper in main crate
                            // Option 3: Generate different arms for the macro?

                            // --- Let's stick to the simple approach for now ---
                            // Modify the generated code to use the `param!` macro logic internally
                            // This reuses the logic we already built for `param!`.
                            let setter_note = $crate::param!($key = $($value)?);

                            // Extract key/value from the generated ParamSetter Note
                            if let $crate::play::Note::ParamSetter { key: resolved_key, value: resolved_value } = setter_note {
                                note = note.param(resolved_key, resolved_value);
                            } else {
                                // This should not happen if param! macro works correctly
                                panic!("Internal error: param! macro did not produce a ParamSetter for key '{}'", stringify!($key));
                            }
                        )*
                        note
                    }};
                }
            }
        };

        // natural duration rest
        let dur_lit = LitFloat::new(&format!("{}f32", base_dur), proc_macro2::Span::call_site());
        let name = format!("r{}", code);
        let ident = Ident::new(&name, proc_macro2::Span::call_site());
        let const_ident = ident.clone(); // Use the same ident for the const
        let doc = format!(
            "Rest `{}` \u{2014} {} of silence. Per-note attrs: `{}!(...)`.",
            const_ident,
            value_words(code, 0),
            const_ident
        );
        tokens.extend(quote! {
            #[doc = #doc]
            pub const #const_ident: Note = Note::Rest { duration: #dur_lit, parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident, &const_ident, doc.clone()));

        // dotted duration rest
        let dot_dur = base_dur * 1.5;
        let dot_lit = LitFloat::new(&format!("{}f32", dot_dur), proc_macro2::Span::call_site());
        let name_dot = format!("r{}d", code);
        let ident_dot = Ident::new(&name_dot, proc_macro2::Span::call_site());
        let const_ident_dot = ident_dot.clone();
        let doc = format!(
            "Rest `{}` \u{2014} {} of silence. Per-note attrs: `{}!(...)`.",
            const_ident_dot,
            value_words(code, 1),
            const_ident_dot
        );
        tokens.extend(quote! {
            #[doc = #doc]
            pub const #const_ident_dot: Note = Note::Rest { duration: #dot_lit, parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_dot, &const_ident_dot, doc.clone()));

        // double-dotted duration rest
        let ddot_dur = base_dur * 1.75;
        let ddot_lit = LitFloat::new(&format!("{}f32", ddot_dur), proc_macro2::Span::call_site());
        let name_ddot = format!("r{}dd", code);
        let ident_ddot = Ident::new(&name_ddot, proc_macro2::Span::call_site());
        let const_ident_ddot = ident_ddot.clone();
        let doc = format!(
            "Rest `{}` \u{2014} {} of silence. Per-note attrs: `{}!(...)`.",
            const_ident_ddot,
            value_words(code, 2),
            const_ident_ddot
        );
        tokens.extend(quote! {
            #[doc = #doc]
            pub const #const_ident_ddot: Note = Note::Rest { duration: #ddot_lit, parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_ddot, &const_ident_ddot, doc.clone()));

        // triple-dotted duration rest
        let tdot_dur = base_dur * 1.875;
        let tdot_lit = LitFloat::new(&format!("{}f32", tdot_dur), proc_macro2::Span::call_site());
        let name_tdot = format!("r{}ddd", code);
        let ident_tdot = Ident::new(&name_tdot, proc_macro2::Span::call_site());
        let const_ident_tdot = ident_tdot.clone();
        let doc = format!(
            "Rest `{}` \u{2014} {} of silence. Per-note attrs: `{}!(...)`.",
            const_ident_tdot,
            value_words(code, 3),
            const_ident_tdot
        );
        tokens.extend(quote! {
            #[doc = #doc]
            pub const #const_ident_tdot: Note = Note::Rest { duration: #tdot_lit, parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_tdot, &const_ident_tdot, doc.clone()));
    }
    tokens.into()
}

// --- Macro 2: Generate Duration Tie Constants (q, hd, etc.) ---
#[proc_macro]
pub fn generate_duration_ties(_input: TokenStream) -> TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();
    for &(code, base_dur) in &DURATIONS {
        let dur_lit = LitFloat::new(&format!("{}f32", base_dur), proc_macro2::Span::call_site());
        let ident = Ident::new(code, proc_macro2::Span::call_site());
        let doc = format!(
            "Duration tie `{}` \u{2014} {}: extends the previous note or rest.",
            ident,
            value_words(code, 0)
        );
        tokens
            .extend(quote! { #[doc = #doc] pub const #ident: Note = Note::DurationTie { duration: #dur_lit }; });

        let dot_dur = base_dur * 1.5;
        let dot_lit = LitFloat::new(&format!("{}f32", dot_dur), proc_macro2::Span::call_site());
        let ident_dot = Ident::new(&format!("{}d", code), proc_macro2::Span::call_site());
        let doc = format!(
            "Duration tie `{}` \u{2014} {}: extends the previous note or rest.",
            ident_dot,
            value_words(code, 1)
        );
        tokens.extend(
            quote! { #[doc = #doc] pub const #ident_dot: Note = Note::DurationTie { duration: #dot_lit }; },
        );

        let ddot_dur = base_dur * 1.75;
        let ddot_lit = LitFloat::new(&format!("{}f32", ddot_dur), proc_macro2::Span::call_site());
        let ident_ddot = Ident::new(&format!("{}dd", code), proc_macro2::Span::call_site());
        let doc = format!(
            "Duration tie `{}` \u{2014} {}: extends the previous note or rest.",
            ident_ddot,
            value_words(code, 2)
        );
        tokens.extend(
            quote! { #[doc = #doc] pub const #ident_ddot: Note = Note::DurationTie { duration: #ddot_lit }; },
        );

        let tdot_dur = base_dur * 1.875;
        let tdot_lit = LitFloat::new(&format!("{}f32", tdot_dur), proc_macro2::Span::call_site());
        let ident_tdot = Ident::new(&format!("{}ddd", code), proc_macro2::Span::call_site());
        let doc = format!(
            "Duration tie `{}` \u{2014} {}: extends the previous note or rest.",
            ident_tdot,
            value_words(code, 3)
        );
        tokens.extend(
            quote! { #[doc = #doc] pub const #ident_tdot: Note = Note::DurationTie { duration: #tdot_lit }; },
        );
    }
    tokens.into()
}

// --- Macro 3: Generate Previous Pitch Constants (p, pq, phd, etc.) ---
#[proc_macro]
pub fn generate_previous_pitch_notes(_input: TokenStream) -> TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();

    // Function to generate the macro_rules! definition (same as above)
    let generate_macro = |ident: &Ident,
                          const_ident: &Ident,
                          doc: String|
     -> proc_macro2::TokenStream {
        let name = ident.to_string();
        let macrodock = format!(
                "{doc}\n\nMacro form \u{2014} attach per-note attributes, e.g. `{name}!(velocity = 90)`.",
            );
        quote! {
            #[macro_export]
            #[doc = #macrodock]
            macro_rules! #ident {
                () => { $crate::play::note::#const_ident };
                ($($key:ident = $($value:expr)? ),* $(,)?) => {{
                    let mut note = $crate::play::note::#const_ident.clone();
                    $(
                        let setter_note = $crate::param!($key = $($value)?);
                        if let $crate::play::Note::ParamSetter { key: resolved_key, value: resolved_value } = setter_note {
                            note = note.param(resolved_key, resolved_value);
                        } else {
                             panic!("Internal error: param! macro did not produce a ParamSetter for key '{}'", stringify!($key));
                        }
                    )*
                    note
                }};
            }
        }
    };

    // Generate base 'p' constant (no duration)
    let p_ident = Ident::new("p", proc_macro2::Span::call_site());
    let const_p_ident = p_ident.clone();
    let doc = format!(
        "Previous-pitch note `{}` \u{2014} repeats the last pitch and takes the previous duration.",
        const_p_ident
    );
    tokens.extend(quote! {
        #[doc = #doc]
        pub const #const_p_ident: Note = Note::PreviousPitch { optional_duration: None, parameters: Vec::new() };
    });
    tokens.extend(generate_macro(&p_ident, &const_p_ident, doc.clone()));

    // Generate 'p' constants with durations
    for &(code, base_dur) in &DURATIONS {
        let dur_lit = LitFloat::new(&format!("{}f32", base_dur), proc_macro2::Span::call_site());
        let ident = Ident::new(&format!("p{}", code), proc_macro2::Span::call_site());
        let const_ident = ident.clone();
        let doc = format!(
            "Previous-pitch note `{}` \u{2014} repeats the last pitch, {}.",
            const_ident,
            value_words(code, 0)
        );
        tokens.extend(quote! {
            #[doc = #doc]
            pub const #const_ident: Note = Note::PreviousPitch { optional_duration: Some(#dur_lit), parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident, &const_ident, doc.clone()));

        let dot_dur = base_dur * 1.5;
        let dot_lit = LitFloat::new(&format!("{}f32", dot_dur), proc_macro2::Span::call_site());
        let ident_dot = Ident::new(&format!("p{}d", code), proc_macro2::Span::call_site());
        let const_ident_dot = ident_dot.clone();
        let doc = format!(
            "Previous-pitch note `{}` \u{2014} repeats the last pitch, {}.",
            const_ident_dot,
            value_words(code, 1)
        );
        tokens.extend(quote! {
            #[doc = #doc]
            pub const #const_ident_dot: Note = Note::PreviousPitch { optional_duration: Some(#dot_lit), parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_dot, &const_ident_dot, doc.clone()));

        let ddot_dur = base_dur * 1.75;
        let ddot_lit = LitFloat::new(&format!("{}f32", ddot_dur), proc_macro2::Span::call_site());
        let ident_ddot = Ident::new(&format!("p{}dd", code), proc_macro2::Span::call_site());
        let const_ident_ddot = ident_ddot.clone();
        let doc = format!(
            "Previous-pitch note `{}` \u{2014} repeats the last pitch, {}.",
            const_ident_ddot,
            value_words(code, 2)
        );
        tokens.extend(quote! {
            #[doc = #doc]
            pub const #const_ident_ddot: Note = Note::PreviousPitch { optional_duration: Some(#ddot_lit), parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_ddot, &const_ident_ddot, doc.clone()));

        let tdot_dur = base_dur * 1.875;
        let tdot_lit = LitFloat::new(&format!("{}f32", tdot_dur), proc_macro2::Span::call_site());
        let ident_tdot = Ident::new(&format!("p{}ddd", code), proc_macro2::Span::call_site());
        let const_ident_tdot = ident_tdot.clone();
        let doc = format!(
            "Previous-pitch note `{}` \u{2014} repeats the last pitch, {}.",
            const_ident_tdot,
            value_words(code, 3)
        );
        tokens.extend(quote! {
            #[doc = #doc]
            pub const #const_ident_tdot: Note = Note::PreviousPitch { optional_duration: Some(#tdot_lit), parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_tdot, &const_ident_tdot, doc.clone()));
    }
    tokens.into()
}

// --- Macro 4: Generate Pitched Notes for a SINGLE Octave ---
#[proc_macro]
pub fn generate_pitched_notes_octave(input: TokenStream) -> TokenStream {
    // Parse directly as LitInt
    let lit_int = parse_macro_input!(input as LitInt);
    let octave_i32 = match lit_int.base10_parse::<i32>() {
        Ok(o) => o,
        Err(e) => panic!("Failed to parse octave integer literal: {}", e),
    };

    // Validate octave range (-1 to 9 is typical for MIDI representations)
    if !(-1..=9).contains(&octave_i32) {
        panic!(
            "Invalid octave {} provided. Must be between -1 and 9.",
            octave_i32
        );
    }

    // Use i32 for octave name generation if negative
    let oct_name = if octave_i32 < 0 {
        format!("_{}", -octave_i32) // e.g., _1 for octave -1
    } else {
        octave_i32.to_string() // e.g., 4 for octave 4
    };
    let mut tokens = proc_macro2::TokenStream::new();

    // Function to generate the macro_rules! definition (same as above)
    let generate_macro = |ident: &Ident,
                          const_ident: &Ident,
                          doc: String|
     -> proc_macro2::TokenStream {
        let name = ident.to_string();
        let macrodock = format!(
                "{doc}\n\nMacro form \u{2014} attach per-note attributes, e.g. `{name}!(velocity = 90)`.",
            );
        quote! {
            #[macro_export]
            #[doc = #macrodock]
            macro_rules! #ident {
                () => { $crate::play::note::#const_ident };
                ($($key:ident = $($value:expr)? ),* $(,)?) => {{
                    let mut note = $crate::play::note::#const_ident.clone();
                    $(
                        let setter_note = $crate::param!($key = $($value)?);
                        if let $crate::play::Note::ParamSetter { key: resolved_key, value: resolved_value } = setter_note {
                            note = note.param(resolved_key, resolved_value);
                        } else {
                            panic!("Internal error: param! macro did not produce a ParamSetter for key '{}'", stringify!($key));
                        }
                    )*
                    note
                }};
            }
        }
    };

    for (letter, base_pc) in PITCHES {
        for &(acc_suf, acc_val) in &ACCIDENTALS {
            // Use octave_i32 directly in MIDI calculation (already uses i32)
            let midi_i32 = (octave_i32 + 1) * 12 + base_pc as i32 + acc_val as i32;
            if !(0..=127).contains(&midi_i32) {
                continue;
            }
            let midi_u8 = midi_i32 as u8;

            // --- Generate notes with explicit durations (Atom) ---
            for &(code, base_dur) in &DURATIONS {
                // natural duration
                let dur_lit =
                    LitFloat::new(&format!("{}f32", base_dur), proc_macro2::Span::call_site());
                let name = format!("{}{}{}{}", letter, acc_suf, oct_name, code);
                let ident = Ident::new(&name, proc_macro2::Span::call_site());
                let const_ident = ident.clone();
                let acc_sym = match acc_suf {
                    "s" => "#",
                    "f" => "b",
                    "ss" => "##",
                    "ff" => "bb",
                    _ => "",
                };
                let doc = format!(
                    "Note `{}` \u{2014} {}. {}. Per-note attrs: `{}!(...)`.",
                    const_ident,
                    pitch_words(letter, &acc_sym, octave_i32, midi_i32),
                    value_words(code, 0),
                    const_ident
                );
                tokens.extend(quote! {
                    #[doc = #doc]
                    pub const #const_ident: Note = Note::Atom { midi: #midi_u8, duration: #dur_lit, parameters: Vec::new()};
                    });
                tokens.extend(generate_macro(&ident, &const_ident, doc.clone()));

                // dotted duration
                let dot_dur = base_dur * 1.5;
                let dot_lit =
                    LitFloat::new(&format!("{}f32", dot_dur), proc_macro2::Span::call_site());
                let name_dot = format!("{}{}{}{}d", letter, acc_suf, oct_name, code);
                let ident_dot = Ident::new(&name_dot, proc_macro2::Span::call_site());
                let const_ident_dot = ident_dot.clone();
                let acc_sym = match acc_suf {
                    "s" => "#",
                    "f" => "b",
                    "ss" => "##",
                    "ff" => "bb",
                    _ => "",
                };
                let doc = format!(
                    "Note `{}` \u{2014} {}. {}. Per-note attrs: `{}!(...)`.",
                    const_ident_dot,
                    pitch_words(letter, &acc_sym, octave_i32, midi_i32),
                    value_words(code, 1),
                    const_ident_dot
                );
                tokens.extend(quote! {
                    #[doc = #doc]
                    pub const #const_ident_dot: Note = Note::Atom { midi: #midi_u8, duration: #dot_lit, parameters: Vec::new()};
                    });
                tokens.extend(generate_macro(&ident_dot, &const_ident_dot, doc.clone()));

                // double-dotted duration
                let ddot_dur = base_dur * 1.75;
                let ddot_lit =
                    LitFloat::new(&format!("{}f32", ddot_dur), proc_macro2::Span::call_site());
                let name_ddot = format!("{}{}{}{}dd", letter, acc_suf, oct_name, code);
                let ident_ddot = Ident::new(&name_ddot, proc_macro2::Span::call_site());
                let const_ident_ddot = ident_ddot.clone();
                let acc_sym = match acc_suf {
                    "s" => "#",
                    "f" => "b",
                    "ss" => "##",
                    "ff" => "bb",
                    _ => "",
                };
                let doc = format!(
                    "Note `{}` \u{2014} {}. {}. Per-note attrs: `{}!(...)`.",
                    const_ident_ddot,
                    pitch_words(letter, &acc_sym, octave_i32, midi_i32),
                    value_words(code, 2),
                    const_ident_ddot
                );
                tokens.extend(quote! {
                    #[doc = #doc]
                    pub const #const_ident_ddot: Note = Note::Atom { midi: #midi_u8, duration: #ddot_lit, parameters: Vec::new()};
                 });
                tokens.extend(generate_macro(&ident_ddot, &const_ident_ddot, doc.clone()));

                // triple-dotted duration
                let tdot_dur = base_dur * 1.875;
                let tdot_lit =
                    LitFloat::new(&format!("{}f32", tdot_dur), proc_macro2::Span::call_site());
                let name_tdot = format!("{}{}{}{}ddd", letter, acc_suf, oct_name, code);
                let ident_tdot = Ident::new(&name_tdot, proc_macro2::Span::call_site());
                let const_ident_tdot = ident_tdot.clone();
                let acc_sym = match acc_suf {
                    "s" => "#",
                    "f" => "b",
                    "ss" => "##",
                    "ff" => "bb",
                    _ => "",
                };
                let doc = format!(
                    "Note `{}` \u{2014} {}. {}. Per-note attrs: `{}!(...)`.",
                    const_ident_tdot,
                    pitch_words(letter, &acc_sym, octave_i32, midi_i32),
                    value_words(code, 3),
                    const_ident_tdot
                );
                tokens.extend(quote! {
                    #[doc = #doc]
                    pub const #const_ident_tdot: Note = Note::Atom { midi: #midi_u8, duration: #tdot_lit, parameters: Vec::new()};
                 });
                tokens.extend(generate_macro(&ident_tdot, &const_ident_tdot, doc.clone()));
            }
            // --- Generate duration-less variant (AtomImplicitDuration) ---
            let name_implicit = format!("{}{}{}", letter, acc_suf, oct_name);
            let ident_implicit = Ident::new(&name_implicit, proc_macro2::Span::call_site());
            let const_ident_implicit = ident_implicit.clone();
            let acc_sym = match acc_suf {
                "s" => "#",
                "f" => "b",
                "ss" => "##",
                "ff" => "bb",
                _ => "",
            };
            let doc = format!("Note `{}` \u{2014} {}. Takes the previous note or rest duration. Per-note attrs: `{}!(...)`.", const_ident_implicit, pitch_words(letter, &acc_sym, octave_i32, midi_i32), const_ident_implicit);
            tokens.extend(quote! {
                #[doc = #doc]
                pub const #const_ident_implicit: Note = Note::AtomImplicitDuration { midi: #midi_u8, parameters: Vec::new() };
            });
            tokens.extend(generate_macro(
                &ident_implicit,
                &const_ident_implicit,
                doc.clone(),
            ));
        }
    }
    tokens.into()
}

// --- Composition body frontend: whitespace-tolerant item splitter ---
//
// `seq_items!(...)` turns a whitespace- and/or comma-separated note body
// into `vec![...]` without changing semantics: the runtime `ser()`/`par()`
// functions still do all resolution (durations, ties, repeats, attrs).
//
// Accepted forms (equivalent where they overlap):
// - `[a, b, c]` — legacy bracket form, comma-split exactly like before.
// - `( ... )` alone — passed through untouched (legacy single-expr form;
//   a top-level comma inside earns a hint to drop the outer parens).
// - `a, b, c` — comma-separated, no brackets.
// - `a b c` — whitespace-separated. Macro calls (`param!`, `repeat!`,
//   nested `ser!`/`par!`, per-note `!(...)` attrs), `::` paths and
//   control-flow blocks stay glued as single items.
use proc_macro2::{Delimiter, TokenTree};

/// Keywords that never start an item (statements/declarations, plus
/// `else`, which always continues a block construct).
const NEVER_START: [&str; 17] = [
    "let", "use", "mod", "struct", "enum", "trait", "impl", "fn", "pub", "extern", "const",
    "static", "type", "union", "else", "in", "ref",
];

/// Keywords that open a block construct: following tokens stay in the
/// same item until the construct's brace group closes (plus `else`).
const OPENERS: [&str; 8] = [
    "if", "match", "loop", "while", "for", "unsafe", "async", "move",
];

fn is_ident_named(token: &TokenTree, name: &str) -> bool {
    matches!(token, TokenTree::Ident(ident) if ident.to_string() == name)
}

/// Split top-level commas of a bracket group's interior.
fn split_commas(tokens: &[TokenTree]) -> Result<Vec<proc_macro2::TokenStream>, String> {
    let mut items = Vec::new();
    let mut current = Vec::new();
    let mut seen_content = false;
    for token in tokens {
        if matches!(token, TokenTree::Punct(p) if p.as_char() == ',') {
            if current.is_empty() {
                return Err("empty item: stray comma".to_string());
            }
            items.push(current);
            current = Vec::new();
        } else {
            seen_content = true;
            current.push(token.clone());
        }
    }
    if !current.is_empty() || !seen_content {
        // Trailing comma (or wholly empty input) is fine.
        if !current.is_empty() {
            items.push(current);
        }
    }
    Ok(items
        .into_iter()
        .map(|item| item.into_iter().collect::<proc_macro2::TokenStream>())
        .collect())
}

fn top_level_commas(tokens: &[TokenTree]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token, TokenTree::Punct(p) if p.as_char() == ','))
}

/// Split a composition body into item token streams.
fn split_items(tokens: &[TokenTree]) -> Result<Vec<proc_macro2::TokenStream>, String> {
    // Case 1: sole `[...]` group — legacy form, comma-split verbatim.
    if let [TokenTree::Group(group)] = tokens {
        if group.delimiter() == Delimiter::Bracket {
            return split_commas(&group.stream().into_iter().collect::<Vec<_>>());
        }
        // Case 2: sole `(...)`/`{...}` group — legacy single-expr form.
        if group.delimiter() == Delimiter::Parenthesis {
            let inner: Vec<TokenTree> = group.stream().into_iter().collect();
            if top_level_commas(&inner) {
                return Err(
                    "drop the outer parens for multi-note bodies: ser!(c4q, d4q), not ser!((c4q, d4q))"
                        .to_string(),
                );
            }
        }
        let mut only = proc_macro2::TokenStream::new();
        only.extend(tokens.iter().cloned());
        return Ok(vec![only]);
    }
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    // Case 3: general whitespace/comma-tolerant split.
    let mut items: Vec<Vec<TokenTree>> = Vec::new();
    let mut current: Vec<TokenTree> = Vec::new();
    let mut opener = false;
    let mut saw_brace = false;
    let mut index = 0;
    let total = tokens.len();
    // finishes `current`, erroring on empties except a single trailing comma
    let finish = |current: &mut Vec<TokenTree>,
                  items: &mut Vec<Vec<TokenTree>>,
                  last: bool|
     -> Result<(), String> {
        if current.is_empty() {
            if last && !items.is_empty() {
                return Ok(());
            }
            return Err("empty item: stray comma".to_string());
        }
        items.push(std::mem::take(current));
        Ok(())
    };
    while index < total {
        let token = &tokens[index];
        // Exiting opener mode: a fresh identifier/literal/group ends the
        // construct unless it is `else` (which continues it).
        if opener && saw_brace {
            let continues = match token {
                TokenTree::Ident(ident) => ident.to_string() == "else",
                _ => false,
            };
            if !continues {
                match token {
                    // A comma here separates (e.g. `if c {a}, d4q` is two).
                    TokenTree::Punct(p) if p.as_char() == ',' => {
                        opener = false;
                        saw_brace = false;
                        finish(&mut current, &mut items, index + 1 == total)?;
                        index += 1;
                        continue;
                    }
                    // Groups attach (calls on the construct); anything else
                    // resumes normal splitting at this token.
                    TokenTree::Group(_) => {}
                    _ => {
                        opener = false;
                        saw_brace = false;
                        continue; // reprocess this token in normal mode
                    }
                }
            } else {
                saw_brace = false;
            }
        }
        match token {
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                // Inside a block construct (opener mode) a comma stays put
                // (it belongs to the construct); otherwise it ends the item.
                if opener {
                    current.push(token.clone());
                } else {
                    finish(&mut current, &mut items, index + 1 == total)?;
                }
            }
            TokenTree::Ident(ident) => {
                let text = ident.to_string();
                if current.is_empty() {
                    if NEVER_START.contains(&text.as_str()) {
                        return Err(format!("unexpected `{text}` here: expected a note"));
                    }
                    current.push(token.clone());
                    if OPENERS.contains(&text.as_str()) {
                        opener = true;
                        saw_brace = false;
                    }
                } else if text == "else"
                    || text == "as"
                    || is_ident_named(current.last().unwrap(), "as")
                    || matches!(current.last(), Some(TokenTree::Punct(p)) if p.as_char() == ':' || p.as_char() == '.')
                    || NEVER_START.contains(&prev_ident(&current).as_str())
                    || opener
                {
                    // Path segments (`a::b`, `.c`), `as` casts, `else`
                    // continuations, keywords inside constructs, and opener
                    // bodies stay glued to the current item.
                    current.push(token.clone());
                } else if matches!(
                    current.last(),
                    Some(TokenTree::Ident(_))
                        | Some(TokenTree::Literal(_))
                        | Some(TokenTree::Group(_))
                ) {
                    // A fresh identifier after a complete item starts a new one.
                    finish(&mut current, &mut items, false)?;
                    if NEVER_START.contains(&text.as_str()) {
                        return Err(format!("unexpected `{text}` here: expected a note"));
                    }
                    current.push(token.clone());
                    if OPENERS.contains(&text.as_str()) {
                        opener = true;
                        saw_brace = false;
                    }
                } else {
                    current.push(token.clone());
                }
            }
            TokenTree::Group(group) => {
                if group.delimiter() == Delimiter::Brace {
                    saw_brace = true;
                }
                current.push(token.clone());
            }
            _ => {
                // Literals start items; any other leading punct must be a
                // unary/ref operator (`-4`, `&x`, `*x`, `!x`); anything else
                // is rejected so typos fail here, not downstream.
                if current.is_empty() {
                    match token {
                        TokenTree::Literal(_) => current.push(token.clone()),
                        TokenTree::Punct(p)
                            if ['-', '+', '&', '*', '!', '~'].contains(&p.as_char()) =>
                        {
                            current.push(token.clone())
                        }
                        _ => {
                            return Err(format!(
                                "expected a note, macro call or Group here, found `{token}`"
                            ));
                        }
                    }
                } else {
                    current.push(token.clone());
                }
            }
        }
        index += 1;
    }
    if !current.is_empty() {
        items.push(current);
    }
    Ok(items
        .into_iter()
        .map(|item| item.into_iter().collect::<proc_macro2::TokenStream>())
        .collect())
}

/// Text of the trailing identifier in `current`, or empty when none.
fn prev_ident(current: &[TokenTree]) -> String {
    match current.last() {
        Some(TokenTree::Ident(ident)) => ident.to_string(),
        _ => String::new(),
    }
}

/// Turn a composition body into `vec![...]` for `ser()`/`par()`/… .
#[proc_macro]
pub fn seq_items(input: TokenStream) -> TokenStream {
    let tokens: Vec<TokenTree> = proc_macro2::TokenStream::from(input).into_iter().collect();
    match split_items(&tokens) {
        Ok(items) => quote! { ::std::vec![#(#items),*] }.into(),
        Err(message) => quote! { ::core::compile_error!(#message) }.into(),
    }
}

#[cfg(test)]
mod seq_tests {
    use super::{split_items, TokenTree};

    fn split(input: &str) -> Result<Vec<String>, String> {
        let tokens: Vec<TokenTree> = input
            .parse::<proc_macro2::TokenStream>()
            .unwrap()
            .into_iter()
            .collect();
        split_items(&tokens).map(|items| items.iter().map(|item| item.to_string()).collect())
    }

    fn expect(input: &str, want: &[&str]) {
        match split(input) {
            Ok(got) => {
                let want: Vec<String> = want.iter().map(|s| s.to_string()).collect();
                // Compare via re-parsed normalization (spacing-insensitive).
                let norm = |v: Vec<String>| {
                    v.into_iter()
                        .map(|s| s.parse::<proc_macro2::TokenStream>().unwrap().to_string())
                        .collect::<Vec<_>>()
                };
                assert_eq!(norm(got), norm(want), "input {input:?}");
            }
            Err(err) => panic!("input {input:?} unexpectedly failed: {err}"),
        }
    }

    fn expect_err(input: &str) {
        assert!(split(input).is_err(), "input {input:?} should fail");
    }

    #[test]
    fn whitespace_and_comma_forms_agree() {
        expect("c4q d4q", &["c4q", "d4q"]);
        expect("c4q, d4q", &["c4q", "d4q"]);
        expect("[c4q, d4q]", &["c4q", "d4q"]);
        expect("c4q", &["c4q"]);
        expect("", &[]);
        expect("c4q,", &["c4q"]);
    }

    #[test]
    fn macro_units_stay_whole() {
        expect("param!(tempo = 120) c4q", &["param!(tempo = 120)", "c4q"]);
        expect("d6o!(velocity = 8.47)", &["d6o!(velocity = 8.47)"]);
        expect("c4i, o, ro", &["c4i", "o", "ro"]);
        expect("repeat!(25)", &["repeat!(25)"]);
        expect("ser!(c4q d4q) e4q", &["ser!(c4q d4q)", "e4q"]);
    }

    #[test]
    fn paths_calls_and_blocks() {
        expect("voice_bass() c4q", &["voice_bass()", "c4q"]);
        expect("voice_bass.clone() c4q", &["voice_bass.clone()", "c4q"]);
        expect("crate::songs::x() c4q", &["crate::songs::x()", "c4q"]);
        expect(
            "if c { a } else { b } d4q",
            &["if c { a } else { b }", "d4q"],
        );
        expect("match x { _ => a } d4q", &["match x { _ => a }", "d4q"]);
        expect("foo as Bar", &["foo as Bar"]);
    }

    #[test]
    fn bad_inputs_fail_here() {
        expect_err(",c4q");
        expect_err("c4q,,d4q");
        expect_err("(c4q, d4q)");
        expect_err("#c4q");
        expect_err("let x = 1");
    }
}

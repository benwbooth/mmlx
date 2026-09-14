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

// --- Macro 1: Generate Rest Notes (rw, rhd, etc.) ---
#[proc_macro]
pub fn generate_rest_notes(_input: TokenStream) -> TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();
    for &(code, base_dur) in &DURATIONS {
        // Function to generate the macro_rules! definition
        let generate_macro = |ident: &Ident, const_ident: &Ident| -> proc_macro2::TokenStream {
            quote! {
                #[macro_export]
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
        tokens.extend(quote! {
            pub const #const_ident: Note = Note::Rest { duration: #dur_lit, parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident, &const_ident));

        // dotted duration rest
        let dot_dur = base_dur * 1.5;
        let dot_lit = LitFloat::new(&format!("{}f32", dot_dur), proc_macro2::Span::call_site());
        let name_dot = format!("r{}d", code);
        let ident_dot = Ident::new(&name_dot, proc_macro2::Span::call_site());
        let const_ident_dot = ident_dot.clone();
        tokens.extend(quote! {
            pub const #const_ident_dot: Note = Note::Rest { duration: #dot_lit, parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_dot, &const_ident_dot));

        // double-dotted duration rest
        let ddot_dur = base_dur * 1.75;
        let ddot_lit = LitFloat::new(&format!("{}f32", ddot_dur), proc_macro2::Span::call_site());
        let name_ddot = format!("r{}dd", code);
        let ident_ddot = Ident::new(&name_ddot, proc_macro2::Span::call_site());
        let const_ident_ddot = ident_ddot.clone();
        tokens.extend(quote! {
            pub const #const_ident_ddot: Note = Note::Rest { duration: #ddot_lit, parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_ddot, &const_ident_ddot));

        // triple-dotted duration rest
        let tdot_dur = base_dur * 1.875;
        let tdot_lit = LitFloat::new(&format!("{}f32", tdot_dur), proc_macro2::Span::call_site());
        let name_tdot = format!("r{}ddd", code);
        let ident_tdot = Ident::new(&name_tdot, proc_macro2::Span::call_site());
        let const_ident_tdot = ident_tdot.clone();
        tokens.extend(quote! {
            pub const #const_ident_tdot: Note = Note::Rest { duration: #tdot_lit, parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_tdot, &const_ident_tdot));
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
        tokens
            .extend(quote! { pub const #ident: Note = Note::DurationTie { duration: #dur_lit }; });

        let dot_dur = base_dur * 1.5;
        let dot_lit = LitFloat::new(&format!("{}f32", dot_dur), proc_macro2::Span::call_site());
        let ident_dot = Ident::new(&format!("{}d", code), proc_macro2::Span::call_site());
        tokens.extend(
            quote! { pub const #ident_dot: Note = Note::DurationTie { duration: #dot_lit }; },
        );

        let ddot_dur = base_dur * 1.75;
        let ddot_lit = LitFloat::new(&format!("{}f32", ddot_dur), proc_macro2::Span::call_site());
        let ident_ddot = Ident::new(&format!("{}dd", code), proc_macro2::Span::call_site());
        tokens.extend(
            quote! { pub const #ident_ddot: Note = Note::DurationTie { duration: #ddot_lit }; },
        );

        let tdot_dur = base_dur * 1.875;
        let tdot_lit = LitFloat::new(&format!("{}f32", tdot_dur), proc_macro2::Span::call_site());
        let ident_tdot = Ident::new(&format!("{}ddd", code), proc_macro2::Span::call_site());
        tokens.extend(
            quote! { pub const #ident_tdot: Note = Note::DurationTie { duration: #tdot_lit }; },
        );
    }
    tokens.into()
}

// --- Macro 3: Generate Previous Pitch Constants (p, pq, phd, etc.) ---
#[proc_macro]
pub fn generate_previous_pitch_notes(_input: TokenStream) -> TokenStream {
    let mut tokens = proc_macro2::TokenStream::new();

    // Function to generate the macro_rules! definition (same as above)
    let generate_macro = |ident: &Ident, const_ident: &Ident| -> proc_macro2::TokenStream {
        quote! {
            #[macro_export]
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
    tokens.extend(quote! {
        pub const #const_p_ident: Note = Note::PreviousPitch { optional_duration: None, parameters: Vec::new() };
    });
    tokens.extend(generate_macro(&p_ident, &const_p_ident));

    // Generate 'p' constants with durations
    for &(code, base_dur) in &DURATIONS {
        let dur_lit = LitFloat::new(&format!("{}f32", base_dur), proc_macro2::Span::call_site());
        let ident = Ident::new(&format!("p{}", code), proc_macro2::Span::call_site());
        let const_ident = ident.clone();
        tokens.extend(quote! {
            pub const #const_ident: Note = Note::PreviousPitch { optional_duration: Some(#dur_lit), parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident, &const_ident));

        let dot_dur = base_dur * 1.5;
        let dot_lit = LitFloat::new(&format!("{}f32", dot_dur), proc_macro2::Span::call_site());
        let ident_dot = Ident::new(&format!("p{}d", code), proc_macro2::Span::call_site());
        let const_ident_dot = ident_dot.clone();
        tokens.extend(quote! {
            pub const #const_ident_dot: Note = Note::PreviousPitch { optional_duration: Some(#dot_lit), parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_dot, &const_ident_dot));

        let ddot_dur = base_dur * 1.75;
        let ddot_lit = LitFloat::new(&format!("{}f32", ddot_dur), proc_macro2::Span::call_site());
        let ident_ddot = Ident::new(&format!("p{}dd", code), proc_macro2::Span::call_site());
        let const_ident_ddot = ident_ddot.clone();
        tokens.extend(quote! {
            pub const #const_ident_ddot: Note = Note::PreviousPitch { optional_duration: Some(#ddot_lit), parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_ddot, &const_ident_ddot));

        let tdot_dur = base_dur * 1.875;
        let tdot_lit = LitFloat::new(&format!("{}f32", tdot_dur), proc_macro2::Span::call_site());
        let ident_tdot = Ident::new(&format!("p{}ddd", code), proc_macro2::Span::call_site());
        let const_ident_tdot = ident_tdot.clone();
        tokens.extend(quote! {
            pub const #const_ident_tdot: Note = Note::PreviousPitch { optional_duration: Some(#tdot_lit), parameters: Vec::new() };
        });
        tokens.extend(generate_macro(&ident_tdot, &const_ident_tdot));
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
    let generate_macro = |ident: &Ident, const_ident: &Ident| -> proc_macro2::TokenStream {
        quote! {
            #[macro_export]
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
                tokens.extend(quote! {
                    pub const #const_ident: Note = Note::Atom { midi: #midi_u8, duration: #dur_lit, parameters: Vec::new()};
                    });
                tokens.extend(generate_macro(&ident, &const_ident));

                // dotted duration
                let dot_dur = base_dur * 1.5;
                let dot_lit =
                    LitFloat::new(&format!("{}f32", dot_dur), proc_macro2::Span::call_site());
                let name_dot = format!("{}{}{}{}d", letter, acc_suf, oct_name, code);
                let ident_dot = Ident::new(&name_dot, proc_macro2::Span::call_site());
                let const_ident_dot = ident_dot.clone();
                tokens.extend(quote! {
                    pub const #const_ident_dot: Note = Note::Atom { midi: #midi_u8, duration: #dot_lit, parameters: Vec::new()};
                    });
                tokens.extend(generate_macro(&ident_dot, &const_ident_dot));

                // double-dotted duration
                let ddot_dur = base_dur * 1.75;
                let ddot_lit =
                    LitFloat::new(&format!("{}f32", ddot_dur), proc_macro2::Span::call_site());
                let name_ddot = format!("{}{}{}{}dd", letter, acc_suf, oct_name, code);
                let ident_ddot = Ident::new(&name_ddot, proc_macro2::Span::call_site());
                let const_ident_ddot = ident_ddot.clone();
                tokens.extend(quote! {
                    pub const #const_ident_ddot: Note = Note::Atom { midi: #midi_u8, duration: #ddot_lit, parameters: Vec::new()};
                 });
                tokens.extend(generate_macro(&ident_ddot, &const_ident_ddot));

                // triple-dotted duration
                let tdot_dur = base_dur * 1.875;
                let tdot_lit =
                    LitFloat::new(&format!("{}f32", tdot_dur), proc_macro2::Span::call_site());
                let name_tdot = format!("{}{}{}{}ddd", letter, acc_suf, oct_name, code);
                let ident_tdot = Ident::new(&name_tdot, proc_macro2::Span::call_site());
                let const_ident_tdot = ident_tdot.clone();
                tokens.extend(quote! {
                    pub const #const_ident_tdot: Note = Note::Atom { midi: #midi_u8, duration: #tdot_lit, parameters: Vec::new()};
                 });
                tokens.extend(generate_macro(&ident_tdot, &const_ident_tdot));
            }
            // --- Generate duration-less variant (AtomImplicitDuration) ---
            let name_implicit = format!("{}{}{}", letter, acc_suf, oct_name);
            let ident_implicit = Ident::new(&name_implicit, proc_macro2::Span::call_site());
            let const_ident_implicit = ident_implicit.clone();
            tokens.extend(quote! {
                pub const #const_ident_implicit: Note = Note::AtomImplicitDuration { midi: #midi_u8, parameters: Vec::new() };
            });
            tokens.extend(generate_macro(&ident_implicit, &const_ident_implicit));
        }
    }
    tokens.into()
}

// ... (Placeholder for other macros) ...

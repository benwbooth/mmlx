// mmlx-core/src/note.rs

// Import Note type from the parent crate (mmlx-core)
use crate::types::Note;
// Import the procedural macros from the mmlx-macros crate (dependency of this crate)
use mmlx_macros::{
    generate_duration_ties, generate_pitched_notes_octave, generate_previous_pitch_notes,
    generate_rest_notes,
};

// Call the individual generation macros
generate_rest_notes!();
generate_duration_ties!();
generate_previous_pitch_notes!();

// Call generate_pitched_notes_octave for each required octave
generate_pitched_notes_octave!(-1);
generate_pitched_notes_octave!(0);
generate_pitched_notes_octave!(1);
generate_pitched_notes_octave!(2);
generate_pitched_notes_octave!(3);
generate_pitched_notes_octave!(4);
generate_pitched_notes_octave!(5);
generate_pitched_notes_octave!(6);
generate_pitched_notes_octave!(7);
generate_pitched_notes_octave!(8);
generate_pitched_notes_octave!(9);

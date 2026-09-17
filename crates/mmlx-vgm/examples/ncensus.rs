//! Temporary VGM feature census: which YM/PSG features the song uses.
//!
//! Usage: `cargo run -p mmlx-vgm --example ncensus -- <song.vgm>`
use mmlx_vgm::{parse_commands, parse_header, Command};
use std::collections::{HashMap, HashSet};

fn main() {
    let path = std::env::args().nth(1).expect("usage: ncensus <song.vgm>");
    let data = std::fs::read(path).expect("read VGM");
    let header = parse_header(&data).expect("header");
    let commands = parse_commands(&data, &header).expect("commands");
    let mut lfo_vals = HashSet::new();
    let mut b4_vals = HashSet::new();
    let mut ssg_vals = HashSet::new();
    let mut dt_vals = HashSet::new();
    let mut noise_vals = HashSet::new();
    let mut dac_writes = 0usize;
    let mut dac_enable = HashSet::new();
    let mut mode_vals = HashSet::new();
    let mut ym_total = 0usize;
    let mut psg_total = 0usize;
    let mut fnum_writes: HashMap<(u8, u8), usize> = HashMap::new();
    for (_, _, cmd) in &commands {
        match cmd {
            Command::YmWrite { port, addr, val } => {
                ym_total += 1;
                match *addr {
                    0x22 => {
                        lfo_vals.insert(*val);
                    }
                    0xB4..=0xB6 => {
                        b4_vals.insert(*val);
                    }
                    0x90..=0x9F => {
                        if *val != 0 {
                            ssg_vals.insert((*addr, *val));
                        }
                    }
                    0x30..=0x3E => {
                        if (*val >> 4) & 7 != 0 {
                            dt_vals.insert((*addr, *val));
                        }
                    }
                    0xA0..=0xA6 | 0xA8..=0xAE => {
                        *fnum_writes.entry((*port, *addr)).or_default() += 1;
                    }
                    0x2A => {
                        dac_writes += 1;
                    }
                    0x2B => {
                        dac_enable.insert(*val);
                    }
                    0x27 => {
                        mode_vals.insert(*val);
                    }
                    _ => {}
                }
            }
            Command::PsgWrite(v) => {
                psg_total += 1;
                if v & 0xF0 == 0xE0 {
                    noise_vals.insert(*v);
                }
            }
            Command::DacTick => {
                dac_writes += 1;
            }
            Command::End => {}
        }
    }
    println!("ym writes: {ym_total}, psg writes: {psg_total}");
    println!("LFO 0x22 values: {lfo_vals:?}");
    println!("0xB4 pan/FMS/AMS distinct: {}", b4_vals.len());
    for v in b4_vals.iter().take(12) {
        println!(
            "  B4={v:#04x} fms={} ams={} pan={}",
            v & 7,
            (v >> 4) & 3,
            v >> 6
        );
    }
    println!("nonzero SSG 0x90 writes: {}", ssg_vals.len());
    for (a, v) in ssg_vals.iter().take(8) {
        println!("  SSG {a:#04x}={v:#04x}");
    }
    println!("detune writes: {}", dt_vals.len());
    for (a, v) in dt_vals.iter().take(8) {
        println!("  DT {a:#04x}={v:#04x}");
    }
    println!("PSG noise writes: {noise_vals:?}");
    println!("DAC data writes: {dac_writes}, DAC enable values: {dac_enable:?}");
    println!("mode 0x27 values: {mode_vals:?}");
    let fnum_total: usize = fnum_writes.values().sum();
    println!("FNUM writes: {fnum_total}");
}

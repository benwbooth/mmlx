fn main() {
    use mmlx_vgm::*;
    use std::collections::HashMap;
    let data = std::fs::read("/tmp/ben/alisia/work/05.vgm").expect("read VGM");
    let header = parse_header(&data).expect("header");
    let commands = parse_commands(&data, &header).expect("commands");
    let end = header.total_samples as u64;
    let mut notes = track_fm(&commands, header.ym2612_clock, end);
    notes.sort_by_key(|n| (n.start, n.voice));
    let mut hist: HashMap<u64, usize> = HashMap::new();
    for n in &notes { *hist.entry(n.duration).or_default() += 1; }
    let mut top: Vec<(u64, usize)> = hist.into_iter().collect();
    top.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("FM notes: {}, top durations:", notes.len());
    for (d, c) in top.iter().take(12) { println!("  {d:6} x{c}"); }
    println!("voice1 head:");
    for n in notes.iter().filter(|n| n.voice == 1).take(12) {
        println!("  t={:8} d={:5} midi={:3}", n.start, n.duration, n.midi);
    }
}

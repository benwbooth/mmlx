/// Decompiled `alisia_stage1` (tempo 116.955444, bar = 128 ticks).
/// `alisia_stage1` plays the intro once; `loop_alisia_stage1` is the looping body.
/// Terse form (no brackets, space-separated): one `// bar N` line
/// per bar, lanes in score order (melody on top, drums at the
/// bottom) so parts align vertically like staff systems;
/// `#[rustfmt::skip]` keeps it. Voice programs live in `voice_*()`
/// definitions up top; `seg_*()` phrases are bar-local repeats.
use mmlx_core::prelude::*;

#[rustfmt::skip]
fn voice_melody() -> Note {
    ser!(
        param!(instrument="psg", sn_channel=2)
    )
}

#[rustfmt::skip]
fn voice_lead() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=15, op1_dr=18, op1_mult=3, op1_rr=8, op1_sl=15, op1_sr=0, op1_tl=8, op2_ar=15, op2_dr=15, op2_mult=1, op2_rr=8, op2_sl=0, op2_sr=0, op2_tl=36, op3_ar=13, op3_dr=18, op3_mult=2, op3_rr=8, op3_sl=2, op3_sr=2, op3_tl=28, op4_ar=15, op4_dr=15, op4_mult=1, op4_rr=8, op4_sl=1, op4_sr=0, op4_tl=24, ym_algo=4, ym_channel=4, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead_2() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=18, op1_dr=14, op1_mult=4, op1_rr=5, op1_sl=6, op1_sr=3, op1_tl=26, op2_ar=19, op2_dr=16, op2_mult=8, op2_rr=5, op2_sl=5, op2_sr=2, op2_tl=64, op3_ar=18, op3_dr=4, op3_mult=4, op3_rr=5, op3_sl=2, op3_sr=0, op3_tl=40, op4_ar=15, op4_dr=4, op4_mult=4, op4_rr=8, op4_sl=0, op4_sr=0, op4_tl=19, ym_algo=3, ym_channel=4, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead_3() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=10, op1_mult=1, op1_rr=5, op1_sl=1, op1_sr=0, op1_tl=31, op2_ar=25, op2_dr=11, op2_mult=5, op2_rr=8, op2_sl=5, op2_sr=0, op2_tl=15, op3_ar=28, op3_dr=13, op3_mult=1, op3_rr=6, op3_sl=2, op3_sr=0, op3_tl=47, op4_ar=14, op4_dr=4, op4_mult=1, op4_rr=6, op4_sl=2, op4_sr=0, op4_tl=20, ym_algo=2, ym_channel=4, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead_4() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=12, op1_dr=6, op1_mult=4, op1_rr=2, op1_sl=2, op1_sr=2, op1_tl=32, op2_ar=10, op2_dr=9, op2_mult=2, op2_rr=6, op2_sl=7, op2_sr=0, op2_tl=31, op3_ar=12, op3_dr=6, op3_mult=4, op3_rr=2, op3_sl=2, op3_sr=2, op3_tl=35, op4_ar=10, op4_dr=9, op4_mult=2, op4_rr=6, op4_sl=7, op4_sr=0, op4_tl=31, ym_algo=4, ym_channel=4, ym_feedback=5)
    )
}

#[rustfmt::skip]
fn voice_lead_5() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=28, op1_mult=6, op1_rr=1, op1_sl=3, op1_sr=5, op1_tl=20, op2_ar=24, op2_dr=1, op2_mult=4, op2_rr=4, op2_sl=15, op2_sr=1, op2_tl=20, op3_ar=25, op3_dr=27, op3_mult=1, op3_rr=4, op3_sl=2, op3_sr=1, op3_tl=14, op4_ar=20, op4_dr=28, op4_mult=2, op4_rr=6, op4_sl=3, op4_sr=5, op4_tl=17, ym_algo=3, ym_channel=4, ym_feedback=3)
    )
}

#[rustfmt::skip]
fn voice_lead_6() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=0, op1_mult=8, op1_rr=10, op1_sl=0, op1_sr=0, op1_tl=46, op2_ar=31, op2_dr=0, op2_mult=4, op2_rr=10, op2_sl=0, op2_sr=0, op2_tl=30, op3_ar=31, op3_dr=0, op3_mult=2, op3_rr=10, op3_sl=0, op3_sr=0, op3_tl=30, op4_ar=31, op4_dr=0, op4_mult=2, op4_rr=10, op4_sl=0, op4_sr=0, op4_tl=30, ym_algo=6, ym_channel=4, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead_7() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=16, op1_dr=8, op1_mult=2, op1_rr=7, op1_sl=3, op1_sr=0, op1_tl=31, op2_ar=18, op2_dr=8, op2_mult=2, op2_rr=7, op2_sl=2, op2_sr=0, op2_tl=26, op3_ar=15, op3_dr=2, op3_mult=1, op3_rr=7, op3_sl=3, op3_sr=0, op3_tl=27, op4_ar=18, op4_dr=3, op4_mult=1, op4_rr=7, op4_sl=3, op4_sr=6, op4_tl=22, ym_algo=4, ym_channel=4, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead2() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=15, op1_dr=18, op1_mult=3, op1_rr=8, op1_sl=15, op1_sr=0, op1_tl=8, op2_ar=15, op2_dr=15, op2_mult=1, op2_rr=8, op2_sl=0, op2_sr=0, op2_tl=28, op3_ar=13, op3_dr=18, op3_mult=2, op3_rr=8, op3_sl=2, op3_sr=2, op3_tl=28, op4_ar=15, op4_dr=15, op4_mult=1, op4_rr=8, op4_sl=1, op4_sr=0, op4_tl=16, ym_algo=4, ym_channel=0, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead2_2() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=18, op1_dr=14, op1_mult=4, op1_rr=5, op1_sl=6, op1_sr=3, op1_tl=26, op2_ar=19, op2_dr=16, op2_mult=8, op2_rr=5, op2_sl=5, op2_sr=2, op2_tl=64, op3_ar=18, op3_dr=4, op3_mult=4, op3_rr=5, op3_sl=2, op3_sr=0, op3_tl=40, op4_ar=15, op4_dr=4, op4_mult=4, op4_rr=8, op4_sl=0, op4_sr=0, op4_tl=11, ym_algo=3, ym_channel=0, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead2_3() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=10, op1_mult=1, op1_rr=5, op1_sl=1, op1_sr=0, op1_tl=31, op2_ar=25, op2_dr=11, op2_mult=5, op2_rr=8, op2_sl=5, op2_sr=0, op2_tl=15, op3_ar=28, op3_dr=13, op3_mult=1, op3_rr=6, op3_sl=2, op3_sr=0, op3_tl=47, op4_ar=14, op4_dr=4, op4_mult=1, op4_rr=6, op4_sl=2, op4_sr=0, op4_tl=12, ym_algo=2, ym_channel=0, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead2_4() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=12, op1_dr=6, op1_mult=4, op1_rr=2, op1_sl=2, op1_sr=2, op1_tl=32, op2_ar=10, op2_dr=9, op2_mult=2, op2_rr=6, op2_sl=7, op2_sr=0, op2_tl=23, op3_ar=12, op3_dr=6, op3_mult=4, op3_rr=2, op3_sl=2, op3_sr=2, op3_tl=35, op4_ar=10, op4_dr=9, op4_mult=2, op4_rr=6, op4_sl=7, op4_sr=0, op4_tl=23, ym_algo=4, ym_channel=0, ym_feedback=5)
    )
}

#[rustfmt::skip]
fn voice_lead2_5() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=28, op1_mult=6, op1_rr=1, op1_sl=3, op1_sr=5, op1_tl=20, op2_ar=24, op2_dr=1, op2_mult=4, op2_rr=4, op2_sl=15, op2_sr=1, op2_tl=20, op3_ar=25, op3_dr=27, op3_mult=1, op3_rr=4, op3_sl=2, op3_sr=1, op3_tl=14, op4_ar=20, op4_dr=28, op4_mult=2, op4_rr=6, op4_sl=3, op4_sr=5, op4_tl=9, ym_algo=3, ym_channel=0, ym_feedback=3)
    )
}

#[rustfmt::skip]
fn voice_lead2_6() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=0, op1_mult=8, op1_rr=10, op1_sl=0, op1_sr=0, op1_tl=46, op2_ar=31, op2_dr=0, op2_mult=4, op2_rr=10, op2_sl=0, op2_sr=0, op2_tl=22, op3_ar=31, op3_dr=0, op3_mult=2, op3_rr=10, op3_sl=0, op3_sr=0, op3_tl=22, op4_ar=31, op4_dr=0, op4_mult=2, op4_rr=10, op4_sl=0, op4_sr=0, op4_tl=22, ym_algo=6, ym_channel=0, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_lead2_7() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=16, op1_dr=8, op1_mult=2, op1_rr=7, op1_sl=3, op1_sr=0, op1_tl=31, op2_ar=18, op2_dr=8, op2_mult=2, op2_rr=7, op2_sl=2, op2_sr=0, op2_tl=18, op3_ar=15, op3_dr=2, op3_mult=1, op3_rr=7, op3_sl=3, op3_sr=0, op3_tl=27, op4_ar=18, op4_dr=3, op4_mult=1, op4_rr=7, op4_sl=3, op4_sr=6, op4_tl=14, ym_algo=4, ym_channel=0, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_arp() -> Note {
    ser!(
        param!(instrument="psg", sn_channel=0)
    )
}

#[rustfmt::skip]
fn voice_arp2() -> Note {
    ser!(
        param!(instrument="psg", sn_channel=1)
    )
}

#[rustfmt::skip]
fn voice_harmony3() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=15, op1_dr=12, op1_mult=2, op1_rr=3, op1_sl=10, op1_sr=1, op1_tl=22, op2_ar=13, op2_dr=4, op2_mult=2, op2_rr=8, op2_sl=5, op2_sr=6, op2_tl=22, op3_ar=31, op3_dr=5, op3_mult=8, op3_rr=2, op3_sl=10, op3_sr=6, op3_tl=17, op4_ar=31, op4_dr=13, op4_mult=4, op4_rr=7, op4_sl=10, op4_sr=4, op4_tl=20, ym_algo=4, ym_channel=3, ym_feedback=6)
    )
}

#[rustfmt::skip]
fn voice_harmony3_2() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=18, op1_dr=14, op1_mult=4, op1_rr=5, op1_sl=6, op1_sr=3, op1_tl=26, op2_ar=19, op2_dr=16, op2_mult=8, op2_rr=5, op2_sl=5, op2_sr=2, op2_tl=64, op3_ar=18, op3_dr=4, op3_mult=4, op3_rr=5, op3_sl=2, op3_sr=0, op3_tl=40, op4_ar=15, op4_dr=4, op4_mult=4, op4_rr=8, op4_sl=0, op4_sr=0, op4_tl=15, ym_algo=3, ym_channel=3, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_harmony3_3() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=15, op1_dr=12, op1_mult=2, op1_rr=3, op1_sl=10, op1_sr=1, op1_tl=22, op2_ar=13, op2_dr=4, op2_mult=2, op2_rr=8, op2_sl=5, op2_sr=6, op2_tl=18, op3_ar=31, op3_dr=5, op3_mult=8, op3_rr=2, op3_sl=10, op3_sr=6, op3_tl=17, op4_ar=31, op4_dr=13, op4_mult=4, op4_rr=7, op4_sl=10, op4_sr=4, op4_tl=16, ym_algo=4, ym_channel=3, ym_feedback=6)
    )
}

#[rustfmt::skip]
fn voice_harmony3_4() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=26, op1_dr=8, op1_mult=3, op1_rr=7, op1_sl=13, op1_sr=5, op1_tl=29, op2_ar=29, op2_dr=5, op2_mult=4, op2_rr=4, op2_sl=4, op2_sr=4, op2_tl=29, op3_ar=28, op3_dr=4, op3_mult=1, op3_rr=6, op3_sl=6, op3_sr=2, op3_tl=34, op4_ar=31, op4_dr=10, op4_mult=1, op4_rr=4, op4_sl=1, op4_sr=6, op4_tl=23, ym_algo=0, ym_channel=3, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_harmony3_5() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=10, op1_mult=1, op1_rr=5, op1_sl=1, op1_sr=0, op1_tl=31, op2_ar=25, op2_dr=11, op2_mult=5, op2_rr=8, op2_sl=5, op2_sr=0, op2_tl=15, op3_ar=28, op3_dr=13, op3_mult=1, op3_rr=6, op3_sl=2, op3_sr=0, op3_tl=47, op4_ar=14, op4_dr=4, op4_mult=1, op4_rr=6, op4_sl=2, op4_sr=0, op4_tl=23, ym_algo=2, ym_channel=3, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_harmony3_6() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=28, op1_mult=6, op1_rr=1, op1_sl=3, op1_sr=5, op1_tl=20, op2_ar=24, op2_dr=1, op2_mult=4, op2_rr=4, op2_sl=15, op2_sr=1, op2_tl=20, op3_ar=25, op3_dr=27, op3_mult=1, op3_rr=4, op3_sl=2, op3_sr=1, op3_tl=14, op4_ar=20, op4_dr=28, op4_mult=2, op4_rr=6, op4_sl=3, op4_sr=5, op4_tl=20, ym_algo=3, ym_channel=3, ym_feedback=3)
    )
}

#[rustfmt::skip]
fn voice_harmony3_7() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=23, op1_mult=10, op1_rr=8, op1_sl=14, op1_sr=10, op1_tl=30, op2_ar=25, op2_dr=14, op2_mult=2, op2_rr=8, op2_sl=15, op2_sr=11, op2_tl=34, op3_ar=25, op3_dr=20, op3_mult=6, op3_rr=8, op3_sl=14, op3_sr=11, op3_tl=34, op4_ar=25, op4_dr=14, op4_mult=2, op4_rr=8, op4_sl=15, op4_sr=12, op4_tl=19, ym_algo=4, ym_channel=3, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_harmony3_8() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=6, op1_mult=7, op1_rr=1, op1_sl=0, op1_sr=6, op1_tl=38, op2_ar=31, op2_dr=6, op2_mult=9, op2_rr=1, op2_sl=0, op2_sr=6, op2_tl=55, op3_ar=31, op3_dr=12, op3_mult=1, op3_rr=1, op3_sl=0, op3_sr=6, op3_tl=37, op4_ar=31, op4_dr=12, op4_mult=1, op4_rr=5, op4_sl=0, op4_sr=6, op4_tl=41, ym_algo=2, ym_channel=3, ym_feedback=0)
    )
}

#[rustfmt::skip]
fn voice_harmony3_9() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=28, op1_mult=6, op1_rr=1, op1_sl=3, op1_sr=5, op1_tl=20, op2_ar=24, op2_dr=1, op2_mult=4, op2_rr=4, op2_sl=15, op2_sr=1, op2_tl=20, op3_ar=25, op3_dr=27, op3_mult=1, op3_rr=4, op3_sl=2, op3_sr=1, op3_tl=14, op4_ar=20, op4_dr=28, op4_mult=2, op4_rr=6, op4_sl=3, op4_sr=5, op4_tl=24, ym_algo=3, ym_channel=3, ym_feedback=3)
    )
}

#[rustfmt::skip]
fn voice_harmony2() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=15, op1_dr=12, op1_mult=2, op1_rr=3, op1_sl=10, op1_sr=1, op1_tl=22, op2_ar=13, op2_dr=4, op2_mult=2, op2_rr=8, op2_sl=5, op2_sr=6, op2_tl=18, op3_ar=31, op3_dr=5, op3_mult=8, op3_rr=2, op3_sl=10, op3_sr=6, op3_tl=17, op4_ar=31, op4_dr=13, op4_mult=4, op4_rr=7, op4_sl=10, op4_sr=4, op4_tl=16, ym_algo=4, ym_channel=2, ym_feedback=6)
    )
}

#[rustfmt::skip]
fn voice_harmony2_2() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=10, op1_mult=1, op1_rr=5, op1_sl=1, op1_sr=0, op1_tl=31, op2_ar=25, op2_dr=11, op2_mult=5, op2_rr=8, op2_sl=5, op2_sr=0, op2_tl=15, op3_ar=28, op3_dr=13, op3_mult=1, op3_rr=6, op3_sl=2, op3_sr=0, op3_tl=47, op4_ar=14, op4_dr=4, op4_mult=1, op4_rr=6, op4_sl=2, op4_sr=0, op4_tl=23, ym_algo=2, ym_channel=2, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_harmony2_3() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=28, op1_mult=6, op1_rr=1, op1_sl=3, op1_sr=5, op1_tl=20, op2_ar=24, op2_dr=1, op2_mult=4, op2_rr=4, op2_sl=15, op2_sr=1, op2_tl=20, op3_ar=25, op3_dr=27, op3_mult=1, op3_rr=4, op3_sl=2, op3_sr=1, op3_tl=14, op4_ar=20, op4_dr=28, op4_mult=2, op4_rr=6, op4_sl=3, op4_sr=5, op4_tl=9, ym_algo=3, ym_channel=2, ym_feedback=3)
    )
}

#[rustfmt::skip]
fn voice_harmony2_4() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=10, op1_mult=1, op1_rr=5, op1_sl=1, op1_sr=0, op1_tl=31, op2_ar=25, op2_dr=11, op2_mult=5, op2_rr=8, op2_sl=5, op2_sr=0, op2_tl=15, op3_ar=28, op3_dr=13, op3_mult=1, op3_rr=6, op3_sl=2, op3_sr=0, op3_tl=47, op4_ar=14, op4_dr=4, op4_mult=1, op4_rr=6, op4_sl=2, op4_sr=0, op4_tl=15, ym_algo=2, ym_channel=2, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_harmony2_5() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=6, op1_mult=7, op1_rr=1, op1_sl=0, op1_sr=6, op1_tl=38, op2_ar=31, op2_dr=6, op2_mult=9, op2_rr=1, op2_sl=0, op2_sr=6, op2_tl=55, op3_ar=31, op3_dr=12, op3_mult=1, op3_rr=1, op3_sl=0, op3_sr=6, op3_tl=37, op4_ar=31, op4_dr=12, op4_mult=1, op4_rr=5, op4_sl=0, op4_sr=6, op4_tl=25, ym_algo=2, ym_channel=2, ym_feedback=0)
    )
}

#[rustfmt::skip]
fn voice_harmony2_6() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=28, op1_mult=6, op1_rr=1, op1_sl=3, op1_sr=5, op1_tl=20, op2_ar=24, op2_dr=1, op2_mult=4, op2_rr=4, op2_sl=15, op2_sr=1, op2_tl=20, op3_ar=25, op3_dr=27, op3_mult=1, op3_rr=4, op3_sl=2, op3_sr=1, op3_tl=14, op4_ar=20, op4_dr=28, op4_mult=2, op4_rr=6, op4_sl=3, op4_sr=5, op4_tl=8, ym_algo=3, ym_channel=2, ym_feedback=3)
    )
}

#[rustfmt::skip]
fn voice_harmony() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=25, op1_dr=10, op1_mult=1, op1_rr=5, op1_sl=1, op1_sr=0, op1_tl=31, op2_ar=25, op2_dr=11, op2_mult=5, op2_rr=8, op2_sl=5, op2_sr=0, op2_tl=15, op3_ar=28, op3_dr=13, op3_mult=1, op3_rr=6, op3_sl=2, op3_sr=0, op3_tl=47, op4_ar=14, op4_dr=4, op4_mult=1, op4_rr=6, op4_sl=2, op4_sr=0, op4_tl=15, ym_algo=2, ym_channel=5, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_harmony_2() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=30, op1_dr=14, op1_mult=0, op1_rr=6, op1_sl=11, op1_sr=8, op1_tl=26, op2_ar=24, op2_dr=10, op2_mult=0, op2_rr=6, op2_sl=11, op2_sr=8, op2_tl=34, op3_ar=28, op3_dr=4, op3_mult=0, op3_rr=6, op3_sl=11, op3_sr=8, op3_tl=18, op4_ar=28, op4_dr=5, op4_mult=1, op4_rr=6, op4_sl=11, op4_sr=8, op4_tl=13, ym_algo=0, ym_channel=5, ym_feedback=6)
    )
}

#[rustfmt::skip]
fn voice_harmony_3() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=23, op1_dr=16, op1_mult=2, op1_rr=9, op1_sl=10, op1_sr=7, op1_tl=25, op2_ar=29, op2_dr=6, op2_mult=3, op2_rr=9, op2_sl=2, op2_sr=6, op2_tl=33, op3_ar=26, op3_dr=9, op3_mult=0, op3_rr=7, op3_sl=1, op3_sr=0, op3_tl=28, op4_ar=26, op4_dr=6, op4_mult=1, op4_rr=8, op4_sl=5, op4_sr=5, op4_tl=18, ym_algo=2, ym_channel=5, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=10, op1_dr=3, op1_mult=4, op1_rr=3, op1_sl=10, op1_sr=0, op1_tl=20, op2_ar=11, op2_dr=3, op2_mult=0, op2_rr=4, op2_sl=6, op2_sr=1, op2_tl=16, op3_ar=12, op3_dr=9, op3_mult=7, op3_rr=5, op3_sl=10, op3_sr=4, op3_tl=27, op4_ar=15, op4_dr=8, op4_mult=5, op4_rr=7, op4_sl=3, op4_sr=10, op4_tl=13, ym_algo=3, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_2() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=28, op1_mult=15, op1_rr=6, op1_sl=0, op1_sr=0, op1_tl=0, op2_ar=31, op2_dr=29, op2_mult=2, op2_rr=10, op2_sl=6, op2_sr=17, op2_tl=7, op3_ar=31, op3_dr=12, op3_mult=15, op3_rr=8, op3_sl=15, op3_sr=8, op3_tl=0, op4_ar=31, op4_dr=25, op4_mult=9, op4_rr=12, op4_sl=5, op4_sr=19, op4_tl=7, ym_algo=4, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_3() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=28, op1_mult=15, op1_rr=6, op1_sl=0, op1_sr=0, op1_tl=0, op2_ar=31, op2_dr=25, op2_mult=2, op2_rr=8, op2_sl=3, op2_sr=12, op2_tl=10, op3_ar=31, op3_dr=12, op3_mult=15, op3_rr=8, op3_sl=15, op3_sr=8, op3_tl=0, op4_ar=31, op4_dr=25, op4_mult=9, op4_rr=12, op4_sl=5, op4_sr=19, op4_tl=13, ym_algo=4, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_4() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=28, op1_mult=15, op1_rr=6, op1_sl=0, op1_sr=0, op1_tl=0, op2_ar=31, op2_dr=29, op2_mult=2, op2_rr=10, op2_sl=6, op2_sr=17, op2_tl=11, op3_ar=31, op3_dr=12, op3_mult=15, op3_rr=8, op3_sl=15, op3_sr=8, op3_tl=0, op4_ar=31, op4_dr=25, op4_mult=9, op4_rr=12, op4_sl=5, op4_sr=19, op4_tl=11, ym_algo=4, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_5() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=8, op1_mult=4, op1_rr=6, op1_sl=13, op1_sr=16, op1_tl=0, op2_ar=31, op2_dr=17, op2_mult=2, op2_rr=9, op2_sl=3, op2_sr=15, op2_tl=11, op3_ar=31, op3_dr=21, op3_mult=4, op3_rr=5, op3_sl=8, op3_sr=21, op3_tl=14, op4_ar=31, op4_dr=16, op4_mult=2, op4_rr=8, op4_sl=10, op4_sr=19, op4_tl=15, ym_algo=4, ym_channel=1, ym_feedback=6)
    )
}

#[rustfmt::skip]
fn voice_bass_6() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=28, op1_mult=15, op1_rr=6, op1_sl=0, op1_sr=0, op1_tl=0, op2_ar=31, op2_dr=25, op2_mult=2, op2_rr=8, op2_sl=3, op2_sr=12, op2_tl=14, op3_ar=31, op3_dr=12, op3_mult=15, op3_rr=8, op3_sl=15, op3_sr=8, op3_tl=0, op4_ar=31, op4_dr=25, op4_mult=9, op4_rr=12, op4_sl=5, op4_sr=19, op4_tl=17, ym_algo=4, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_7() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=8, op1_mult=4, op1_rr=6, op1_sl=13, op1_sr=16, op1_tl=0, op2_ar=31, op2_dr=17, op2_mult=2, op2_rr=9, op2_sl=3, op2_sr=15, op2_tl=7, op3_ar=31, op3_dr=21, op3_mult=4, op3_rr=5, op3_sl=8, op3_sr=21, op3_tl=14, op4_ar=31, op4_dr=16, op4_mult=2, op4_rr=8, op4_sl=10, op4_sr=19, op4_tl=11, ym_algo=4, ym_channel=1, ym_feedback=6)
    )
}

#[rustfmt::skip]
fn voice_bass_8() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=29, op1_dr=31, op1_mult=4, op1_rr=5, op1_sl=0, op1_sr=20, op1_tl=20, op2_ar=29, op2_dr=31, op2_mult=0, op2_rr=7, op2_sl=0, op2_sr=18, op2_tl=20, op3_ar=29, op3_dr=31, op3_mult=0, op3_rr=6, op3_sl=0, op3_sr=14, op3_tl=11, op4_ar=31, op4_dr=31, op4_mult=0, op4_rr=9, op4_sl=0, op4_sr=15, op4_tl=15, ym_algo=2, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_9() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=8, op1_mult=4, op1_rr=6, op1_sl=13, op1_sr=16, op1_tl=0, op2_ar=31, op2_dr=17, op2_mult=2, op2_rr=9, op2_sl=3, op2_sr=15, op2_tl=8, op3_ar=31, op3_dr=21, op3_mult=4, op3_rr=5, op3_sl=8, op3_sr=21, op3_tl=14, op4_ar=31, op4_dr=16, op4_mult=2, op4_rr=8, op4_sl=10, op4_sr=19, op4_tl=12, ym_algo=4, ym_channel=1, ym_feedback=6)
    )
}

#[rustfmt::skip]
fn voice_bass_10() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=28, op1_mult=15, op1_rr=6, op1_sl=0, op1_sr=0, op1_tl=0, op2_ar=31, op2_dr=25, op2_mult=2, op2_rr=8, op2_sl=3, op2_sr=12, op2_tl=11, op3_ar=31, op3_dr=12, op3_mult=15, op3_rr=8, op3_sl=15, op3_sr=8, op3_tl=0, op4_ar=31, op4_dr=25, op4_mult=9, op4_rr=12, op4_sl=5, op4_sr=19, op4_tl=14, ym_algo=4, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_11() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=31, op1_dr=28, op1_mult=15, op1_rr=6, op1_sl=0, op1_sr=0, op1_tl=0, op2_ar=31, op2_dr=29, op2_mult=2, op2_rr=10, op2_sl=6, op2_sr=17, op2_tl=8, op3_ar=31, op3_dr=12, op3_mult=15, op3_rr=8, op3_sl=15, op3_sr=8, op3_tl=0, op4_ar=31, op4_dr=25, op4_mult=9, op4_rr=12, op4_sl=5, op4_sr=19, op4_tl=8, ym_algo=4, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_12() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=29, op1_dr=31, op1_mult=4, op1_rr=5, op1_sl=0, op1_sr=20, op1_tl=20, op2_ar=29, op2_dr=31, op2_mult=0, op2_rr=7, op2_sl=0, op2_sr=18, op2_tl=20, op3_ar=29, op3_dr=31, op3_mult=0, op3_rr=6, op3_sl=0, op3_sr=14, op3_tl=11, op4_ar=31, op4_dr=31, op4_mult=0, op4_rr=9, op4_sl=0, op4_sr=15, op4_tl=12, ym_algo=2, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_bass_13() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=30, op1_dr=5, op1_mult=4, op1_rr=6, op1_sl=13, op1_sr=16, op1_tl=1, op2_ar=31, op2_dr=17, op2_mult=2, op2_rr=9, op2_sl=3, op2_sr=15, op2_tl=8, op3_ar=31, op3_dr=18, op3_mult=2, op3_rr=5, op3_sl=8, op3_sr=18, op3_tl=4, op4_ar=31, op4_dr=16, op4_mult=0, op4_rr=8, op4_sl=10, op4_sr=19, op4_tl=21, ym_algo=4, ym_channel=1, ym_feedback=6)
    )
}

#[rustfmt::skip]
fn voice_bass_14() -> Note {
    ser!(
        param!(instrument="ym", op1_ar=29, op1_dr=18, op1_mult=1, op1_rr=6, op1_sl=5, op1_sr=0, op1_tl=6, op2_ar=31, op2_dr=26, op2_mult=1, op2_rr=6, op2_sl=2, op2_sr=22, op2_tl=8, op3_ar=31, op3_dr=27, op3_mult=2, op3_rr=6, op3_sl=5, op3_sr=8, op3_tl=13, op4_ar=31, op4_dr=14, op4_mult=1, op4_rr=6, op4_sl=12, op4_sr=17, op4_tl=8, ym_algo=3, ym_channel=1, ym_feedback=7)
    )
}

#[rustfmt::skip]
fn voice_drums() -> Note {
    ser!(
        param!(instrument="psg", sn_channel=3)
    )
}

#[rustfmt::skip]
fn melodyi_seg_0() -> Note {
    ser!(
        ro gs4i o rx e4i o ro gs4i o ro e4i o ro,
    )
}

#[rustfmt::skip]
fn arp2i_seg_0() -> Note {
    ser!(
        rx c5i rx e5i rid g5i rx a4i rx e5i rx c5i rx e5i rid o,
    )
}

#[rustfmt::skip]
pub fn alisia_stage1() -> Note {
    par!(
        param!(tempo=116.955444),
        // melody (psg)
        ser!(
        ridd o voice_melody() param!(velocity=76.2) e3i rx a3i rx e4i rx b4e x ro e3i rx b4i rx f3i rx c4i rx a4e x rx c3t // bar 1
                t rx a4i rx g3i rx d4i rx g4e x rx d3i rxd g4i rx a3i rx e4i rx b4e xd ro e3i rxd // bar 2
                a4i ro a3i o ro e4i o ro b4e xd ro e3i o ro b4i o ro f3i o ro c4i o ro a4e xd ro c3i rx a4i o // bar 3
                ro g3i rx d4i rx g4e x rx d3i rx g4i rx a3i rx e4i rx b4e xd rx e3i o ro param!(velocity=67.73) a4i rx a4td // bar 4
                x o ro f5i o rx b5e xd ro e4i rx b5i o ro f4i o ro b4i rx as5e x rxd c4i rx a5i rx g4i rx d5x // bar 5
                td rx g5e xd ro d4i o ro g5i o ro a4i o ro f5i o ro b5e xd ro ds4i o ro a5i o ro a4i o rx f5i o // bar 6
                ro b5e x rx e4i rx b5i rx f4i rx b4i rx as5e x rx c4i rx a5i rx g4i o ro d5i o ro g5tdd // bar 7
                i o xd ro d4i o rx g5i o ro a4i rx f5i o ro b5e x rxd ds4i rx a5i rx a4i rx f5i rxd b5id // bar 8
                t xd ro e4i rx b5i o ro f4i o ro b4i o ro as5e xd ro c4i o rx a5i o ro g4i o ro d5i o ro g5e xd // bar 9
                ro d4i o ro g5i rxd a4i rx f5i rx b5e x rx ds4i rx a5i rx a4i o ro f5i o rx b5e xd ro e4t o // bar 10
                xd o ro b5i o rx f4i o ro b4i o ro as5e xd rx c4i o ro a5i o ro g4i rx d5i rxd g5e xd ro g4i rx // bar 11
                gs5i rxd cs5i ro f5i o rx b5i o ro b5i o ro d6i o ro a5i o ro gs4i o rx e4i o ro gs4i o ro e4i o ro gs4i o ro e4td // bar 12
                x o ro g4i o ro d4i rxd g4i rx d4i rx g4i rx d4i rx as4i rx ds4i rx as4i o ro ds4i o rx as4i o ro e4i o ro gs4x // bar 13
                td o ro e4i o rx gs4i o ro e4i o ro gs4i o ro ds4i o rx as4i o ro e4i o ro as4i rxd e4i rx as4i rx e4i rx g4tdd // bar 14
                o rxd d4i rx g4i rx d4i rx g4i o ro d4i o ro gs4i o ro e4i o ro gs4i o ro e4i o ro gs4i o ro e4i o rx g4i o ro d4xd // bar 15
                t o o ro g4i o ro d4i o ro g4i rx d4i rx gs4i rxd e4i rx gs4i rx e4i rx gs4i rx e4i o ro g4i o ro d4i o ro // bar 16
                ro g4i o ro d4i o ro g4i o ro d4i o rx as4i o ro ds4i o ro as4i o ro ds4i o rx as4i o ro e4i rx gs4i rx e4i rxd gs4t // bar 17
                t rx e4i rx gs4i rx e4i rx c4i o ro e4i o rx as4i o ro f4i o ro b4i o ro a4i o ro as3i o ro ds4i o rx e4i o ro // bar 18
                d4i o ro b4i o ro f4i o ro ds5i rx b5i rx a5i rx f6i rxd a5i rx as5i rx ds5i ro b5i rxd as5i rx b5xd rt // bar 19
                rxd e6i o rx ds6i o ro a3i o ro e4i o rx b4e xd ro e3i o ro b4i o rx f3i o ro c4i rx a4e xd rx c3o // bar 20
                tdd rx a4i rx g3i rx d4i rxd g4e x ro d3i o rx g4i o ro a3i o ro e4i o ro b4e xd ro e3i // bar 21
                o rx a4i o ro a3i o ro e4i o ro b4e x rx e3i rx b4i rxd f3i rx c4i rx a4e xd ro c3i o ro a4t // bar 22
                t o ro g3i o rx d4i o ro g4e xd ro d3i o rx g4i o ro a3i o ro e4i o rx cs5e xd rx e3i rx a4i // bar 23
                rx d4i rx a3i rx ds4i rxd e4i rx as4i ro e4i o ro g4i o rx d4i o ro gs4i o ro as4i o ro g4i o ro ds4i o ro e4t o // bar 24
                xd o ro c4i o rx e4i o ro as4i o ro e4i o ro c4i rx g4i rx d4i rx gs4i rx as4i rx g4i rx ds4i rx e4i o ro ds4x // bar 25
                td o rx e4i o ro as4i o ro e4i o ro d4i o ro e4i o rx c4i o ro e4i o ro gs4i o ro e4i o rx c4i o ro fs4i rx d4tdd // bar 26
                o rx fs4i rxd a4i rx fs4i rx d4i rx fs4i rx d4i rx fs4i o ro a4i o ro fs4i o ro ds4i o ro e4i o ro ds4i o rx e4xd // bar 27
                t o o ro as4i o ro e4i o ro d4i o ro gs4i o ro e4i o ro gs4i rx b4i rx gs4i rx e4i rxd g4i rx d4i rx gs4i rx // bar 28
                as4i o ro g4i rx d4i o ro g4i o rx d4i o ro gs4i rx as4i o ro g4i o ro d4i o rx g4i o ro d4i o ro gs4i o rx as4t o // bar 29
                xd rx g4i rx d4i rx g4i rx d4i rx gs4i rxd as4i ro g4i o ro d4i o ro a3i o ro e4i o ro b4e xd ro e3xd // bar 30
                t o o ro b4i o ro f3i o ro c4i o ro a4e xd ro c3i rx a4i rx g3i rx d4i rx g4e x rx d3i rx g4o // bar 31
                tdd o ro a3i rx e4i rx b4e xd rx e3i rx a4i rx a3i rx e4i rx b4e x rx e3i rx b4i // bar 32
                rx f3i rx c4i rx a4e x rx c3i rx a4i rx g3i rx d4i o ro g4e xd ro d3i o ro g4i o ro a3td // bar 33
                x o ro e4i o ro b4e xd rx e3i o ro a4i rx a4i o ro f5i rx b5e x rx e4i rx b5i rx f4i rx b4xd // bar 34
                t o rx as5e xd ro c4i o rx a5i rx g4i o ro d5i o ro g5e xd ro d4i rx g5i o rx a4i o ro f5i ro // bar 35
                ro b5e x rx ds4i rx a5i rx a4i rxd f5i rx b5e x ro e4i o ro b5i o rx f4i o ro b4i o ro as5td // bar 36
                i x xd ro c4i o ro a5i o ro g4i o ro d5i o ro g5e x rx d4i rx g5i rx a4i rx f5i rx b5idd // bar 37
                x x rx ds4i o rx a5i o ro a4i o ro f5i o ro b5e xd rx e4i o ro b5i o rx f4i o ro b4i rx as5e xd // bar 38
                ro c4i rx a5i rxd g4i rx d5i rx g5e xd rx d4i rx g5i rx a4i o ro f5i o ro b5e xd ro ds4t o // bar 39
                xd o ro a5i o ro a4i o rx f5i o ro b5e xd ro e4i o ro b5i o ro f4i rx b4i rx as5e x rxd c4i rx a5o // bar 40
                tdd rx g4i rx d5i o ro g5e xd rx g4i o ro gs5i o ro cs5i o ro f5i o rx b5i o ro b5i o rx d6i o ro a5td // bar 41
                x o ro gs4i o ro e4i rxd gs4i rx e4i rx gs4i rx e4i rxd g4i rx d4i rx g4i o ro d4i o ro g4i o ro d4i o ro as4x // bar 42
                td o ro ds4i o ro as4i o rx ds4i o ro as4i o ro e4i o ro gs4i o ro e4i o ro gs4i rxd e4i rx gs4i rx ds4i rx as4i // bar 43
                rx e4i rx as4i rx e4i o ro as4i o rx e4i o ro g4i o ro d4i o ro g4i rxd d4i o ro g4i o ro d4i o ro gs4i o rx e4xd // bar 44
                t o o ro gs4i o ro e4i rx gs4i rx e4i rxd g4i rx d4i rx g4i rx d4i rxd g4i ro d4i o melodyi_seg_0() // bar 45
                gs4i o ro e4i o ro g4i o ro d4i o rx g4i o ro d4i o ro g4i o ro d4i rx as4i rx ds4i rxd as4t o // bar 46
                xd rx ds4i rx as4i rx e4i rx gs4i o ro e4i o melodyi_seg_0() c4i o rx e4i o ro as4i o ro f4o // bar 47
                tdd o rx b4i o ro a4i o ro as3i rx ds4i rxd e4i rx d4i rx b4i rx f4i rx ds5i rx b5i o ro a5i o ro f6tdd // bar 48
                o o rx a5i o ro as5i o ro ds5i o ro b5i o ro as5i o ro b5t rh td // bar 49
                rhd // bar 50
                ),
        // lead (ym)
        ser!(
        rw // bar 1
                rhd idd voice_lead() c3t o e3x g3o c4x e4i // bar 2
                qdd idd rxd e4edd rx f4x ro g4x ro f4o rxd e4idd o // bar 3
                hd o tdd ro voice_lead_2() e3i o ro f3i o rx g3x // bar 4
                td rx a3i rx b3i rx c4i rx b3x ro c4x rx b3q ro a3e xd rx g3e xd ro d3i o // bar 5
                tdd xd ro f3e x rx e3q td rxd e3e x rx e3i rx f3i rx g3i rx a3td // bar 6
                x rx b3i rx c4i o ro d4edd o ro e4i rx c4e xd ro b3e xd ro d3e xd rx g3xd // bar 7
                id o x rx a3q tdd rx a3e x rx param!(op4_tl=23) e4i o ro f4i o ro g4i o ro a4i o rx b4i o // bar 8
                ro c5i o ro b4x rx c5x ro b4q rx a4e xd ro g4e x rx d4e x rxd f4idd o // bar 9
                o x rx e4q td rx e4e xd ro e4i o ro f4i o ro g4i o rx a4i o ro b4i o ro c5i o rx e5o // bar 10
                eddd o tdd ro d5id rx c5x ro d5o rxd c5q tdd rx b4e xd ro a4tdd // bar 11
                ed o i ro voice_lead_3() c5i o ro d5i o rx e5edd rx f5i rx g5e x rxd d5id // bar 12
                e t td rx c5i o ro b4i o ro c5edd o ro d5i o rx e5e xd ro e5e xd // bar 13
                rx b4e xd ro a4x ro b4x rx a4o rx g4i rxd a4edd o ro b4i rxd c5e xd ro d5e xd ro c5t // bar 14
                id xd rx d5e xd ro e5h ed xd // bar 15
                t o xd rx c5i rx d5i rx e5edd o rx f5i o ro g5e xd ro d5q t o // bar 16
                x rx c5i o ro b4i o rx c5edd o ro d5i rxd e5e xd ro e5e x rx b4e xd rx a4o // bar 17
                o ro b4x rx a4o rx g4i o ro a4edd o rx b4i o ro c5e xd ro e5e x rxd d5i rx e5xd rx d5xd rx c5tdd // bar 18
                i o x rx e5qdd xd rx e5qd td // bar 19
                td ro voice_lead_4() e4i o ro f4i o ro g4i o rx a4i rx b4i rx c5i rxd b4q tdd ro a4e xd // bar 20
                rx g4e xd ro d4e xd rx f4e xd ro e4q tdd ro e4e x rxd e4t // bar 21
                t rx f4i rx g4i rx a4i rxd b4i ro c5i o ro d5edd o rx e5i o ro c5e xd ro b4i xd // bar 22
                t o xd rx d4e xd ro g4e x rxd a4q tdd rx voice_lead_2() a2e xd ro d3i o rx e3t o // bar 23
                xd o ro f3i o ro g3i o ro a3i o rx c4i o ro b3x ro c4x ro b3q rx a3i rx g3i o rx a3e t o // bar 24
                tdd rx g3i rx f3e x rx g3q td rx d3i o ro e3i o ro f3edd o // bar 25
                rx g3i o ro f3e xd rx e3t o ro f3t o rx e3t o rx d3e x rx c3e x rxd d3q x // bar 26
                ed x ro d3q tdd ro d3i rx e3i o rx f3edd o ro g3i rx // bar 27
                f3e x rx e3t o rx f3t o ro e3t o rx d3e x rx c3e xd ro d3qd // bar 28
                w // bar 29
                e eddd rx c3i o ro d3i o rx e3qdd xd // bar 30
                qd t o td rh t o // bar 31
                rq i xd voice_lead() c3td e3x g3x c4o e4h i x // bar 32
                t ro e4edd o ro f4x rx g4x ro f4o rx e4h e xd // bar 33
                edd o tdd rx voice_lead_2() e3i rx f3i rx g3i rx a3i o ro b3i rx c4i o ro b3x rx c4o rx b3e tdd // bar 34
                i o rx a3e xd ro g3e xd ro d3e x rx f3e x rxd e3q t // bar 35
                xd ro e3e xd ro e3i o ro f3i o ro g3i o rx a3i o ro b3i o ro c4i rx d4edd o ro e4i rx c4xd // bar 36
                id o x rx b3e x rx d3e x rxd g3e x ro a3q tdd ro a3i xd // bar 37
                t o xd rx param!(op4_tl=23) e4i o ro f4i o ro g4i o rx a4i o ro b4i o ro c5i o ro b4x rx c5x ro b4q rx a4e // bar 38
                x rxd g4e x rx d4e xd ro f4e xd rx e4q tdd ro e4e xd rx e4o // bar 39
                tdd o ro f4i rx g4i rx a4i rx b4i rx c5i rxd e5q td ro d5id rx c5o rxd d5x ro c5i // bar 40
                ed tdd rx b4e xd ro a4q tdd rxd voice_lead_3() c5i rx d5i rx e5id o // bar 41
                idd o o ro f5i o rx g5e xd ro d5q tdd rx c5i rx b4i o ro c5e xd // bar 42
                i o o rx d5i rx e5e x rx e5e x rxd b4e x rx a4x ro b4o rx a4x rx g4i o ro a4ed o // bar 43
                xd o ro b4i o ro c5e xd rx d5e xd ro c5e xd rx d5e x rx e5eddd o // bar 44
                h o t rx c5i o ro d5i o rx e5edd o ro f5td // bar 45
                x o ro g5e xd rx d5q td rx c5i rx b4i rx c5edd rx d5i o rx e5x // bar 46
                idd xd ro e5e xd ro b4e xd rx a4x ro b4o rxd a4x ro g4i o ro a4edd o rx b4i rx c5i // bar 47
                i xd rx e5e x rx d5i o ro e5t rx d5xd rx c5e xd ro e5qd td // bar 48
                td ro e5qdd xd rxd voice_lead_5() e4i rqd xd // bar 49
                rhd // bar 50
                ),
        // lead2 (ym)
        ser!(
        rw // bar 1
                rhd td voice_lead2() c3t o e3x g3x c4x e4idd o // bar 2
                qd o idd o e4edd o ro f4o rxd g4o rx f4x ro e4ed // bar 3
                h ed td rx voice_lead2_2() e3i o rx f3i o ro g3i o ro a3o // bar 4
                tdd o ro b3i o ro c4i o rx b3x ro c4o rx b3q rx a3e x rx g3e x rxd d3e o // bar 5
                o rx f3e x rx e3q tdd ro e3e xd ro e3i o ro f3i o ro g3i rx a3i o ro b3t o // bar 6
                xd o rx c4i rx d4edd rx e4i rx c4e x ro b3e xd rx d3e xd ro g3id // bar 7
                t xd ro a3q tdd rx a3e xd ro e4i rxd f4i rx g4i rx a4i rx b4i rxd c5tdd // bar 8
                o rx b4x ro c5x rx b4q ro a4e xd ro g4e xd ro d4e xd rx f4e xd ro e4t // bar 9
                edd tdd rx e4e x rx e4i rx f4i rx g4i o ro a4i o ro b4i o ro c5i o rx e5i x // bar 10
                e td i ro d5id rx c5o rx d5x ro c5q tdd rx b4e x rxd a4idd o // bar 11
                e o tdd ro voice_lead2_3() c5i o rx d5i o ro e5edd o ro f5i o rx g5e xd ro d5e t o // bar 12
                i xd tdd ro c5i rxd b4i rx c5edd rx d5i rx e5e x rx e5e xd rx b4tdd // bar 13
                i o xd ro a4x rx b4o rxd a4o rx g4i o ro a4edd o rx b4i o ro c5e xd ro d5e xd rx c5id // bar 14
                t xd rx d5e x ro e5hd t // bar 15
                rx c5i o ro d5i rx e5edd rx f5i rxd g5e x rx d5q tdd ro c5t o // bar 16
                xd o ro b4i o rx c5edd o ro d5i o rx e5e xd ro e5e x rxd b4e xd ro a4x rx b4x ro a4o rx // bar 17
                ro g4i rx a4edd o ro b4i o ro c5e xd rx e5e xd ro d5i o ro e5t ro d5xd rx c5e // bar 18
                xd rx e5qdd xd ro e5qdd xd rx voice_lead2_4() e4x // bar 19
                td o ro f4i o rx g4i o ro a4i o ro b4i o ro c5i o rx b4q tdd rx a4e xd ro g4tdd // bar 20
                i o xd rx d4e x rx f4e x rx e4q tdd rx e4e xd ro e4i o ro f4xd // bar 21
                t o o rx g4i rx a4i o ro b4i o ro c5i rx d5edd rxd e5i rx c5e x rx b4e xd // bar 22
                ro d4e xd rx g4e xd ro a4q tdd rx voice_lead2_2() a2e xd rx d3i rx e3i rxd f3xd // bar 23
                t o rx g3i rx a3i rx c4i rxd b3x ro c4x ro b3q rx a3i o ro g3i rx a3edd o ro // bar 24
                g3i o ro f3e xd rx g3q td rx d3i rx e3i rx f3edd rx g3tdd // bar 25
                o o ro f3e xd rx e3t o rx f3t o rx e3t o ro d3e xd rx c3e xd ro d3q i xd // bar 26
                e o rx d3q tdd ro d3i o ro e3i rx f3edd o ro g3i o ro f3i o // bar 27
                tdd xd rx e3t o ro f3t o rx e3t o rx d3e xd ro c3e x rx d3qdd // bar 28
                w // bar 29
                i eddd rx c3i rx d3i rx e3h t // bar 30
                q id td rh idd // bar 31
                rq x voice_lead2() c3t o e3x g3x c4o e4h idd o rx e4xd // bar 32
                ed o rx f4x ro g4x rx f4o rx e4h ed xd // bar 33
                e t o td rx voice_lead2_2() e3i o rx f3i o ro g3i rx a3i rx b3i rx c4i rx b3x ro c4x rx b3eddd o // bar 34
                o ro a3e xd ro g3e xd ro d3e xd rx f3e xd ro e3q tdd rx e3t // bar 35
                id x rx e3i rx f3i rx g3i rx a3i rxd b3i rx c4i ro d4edd o rx e4i o ro c4i xd // bar 36
                t o x rx b3e xd ro d3e xd ro g3e xd ro a3q td rx a3e x ro // bar 37
                ro e4i rx f4i o ro g4i o rx a4i o ro b4i o ro c5i o rx b4x ro c5o rxd b4q rx a4e xd ro g4t // bar 38
                id x rx d4e xd rx f4e x rx e4q tdd rx e4e xd ro e4i o ro // bar 39
                f4i o ro g4i o ro a4i o ro b4i o rx c5i o ro e5q tdd ro d5id rx c5x ro d5o rxd c5e // bar 40
                e tdd ro b4e xd ro a4q i rx voice_lead2_3() c5i o ro d5i o ro e5e td // bar 41
                td o rx f5i rx g5e x rxd d5q tdd rx c5i rx b4i o ro c5ed xd // bar 42
                o o ro d5i o ro e5e xd rx e5e xd ro b4e xd ro a4x rx b4o rx a4x ro g4i rx a4edd rxd b4xd // bar 43
                t o rx c5e x rx d5e xd ro c5e xd rx d5e xd ro e5q i // bar 44
                qdd t rxd c5i rx d5i rx e5edd o ro f5i o rx g5t // bar 45
                id xd ro d5q tdd ro c5i o rx b4i o ro c5edd rx d5i rx e5i xd // bar 46
                t o x rxd e5e x ro b4e xd rx a4x rx b4o rx a4o rx g4i o ro a4edd o rx b4i o rx c5e // bar 47
                xd ro e5e xd rx d5i rx e5xd rx d5xd rx c5e x rxd e5qdd xd ro e5x // bar 48
                qd td xd rx voice_lead2_5() e4i o rqddd // bar 49
                rhd // bar 50
                ),
        // arp (psg)
        ser!(
        voice_arp() param!(velocity=93.13) e3tdd rxd a3i rx e4i rx b4e x rx e3i rx b4i ro f3i rx c4i rx a4e x rx c3i rx a4i ro // bar 1
                ro g3i rx d4i rx g4e x rx d3i rx g4i rx a3i rxd e4i rx b4e x rx e3i rx a4i rx a3td // bar 2
                x rx e4i o ro b4e xd ro e3i o ro b4i o ro f3i o ro c4i o ro a4e xd ro c3i rx a4i rx g3i o ro d4t // bar 3
                t rx g4e x rx d3i rx g4i rx a3i rx e4i rx b4e x rx e3i rxd param!(velocity=84.67) a4i o ro a4i o ro f5i rx b5o // bar 4
                idd o xd rx e4i rx b5i rx f4i rx b4i rx as5e x rx c4i rxd a5i rx g4i rx d5i rx g5tdd // bar 5
                i o x rx d4i rx g5i rx a4i rx f5i rx b5e x rx ds4i rx a5i rx a4i rx f5i rxd b5idd // bar 6
                x xd ro e4i rx b5i rx f4i rx b4i rx as5e x rx c4i rx a5i rx g4i rx d5i rx g5e xd ro d4x // bar 7
                td rx g5i rxd a4i rx f5i rx b5e xd ro ds4i rxd a5i rx a4i rx f5i rx b5e x rxd e4tdd // bar 8
                o rx b5i rx f4i rx b4i rx as5e x rx c4i rxd a5i rx g4i rx d5i rx g5e xd ro d4i rx g5t // bar 9
                t rxd a4i rx f5i rx b5e x rx ds4i rx a5i rx a4i rx f5i rx b5e xd rx e4i rx b5i rx // bar 10
                f4i rxd b4i rx as5e xd ro c4i rxd a5i rx g4i rx d5i rx g5e x rxd g4i rx gs5i rx cs5t o // bar 11
                xd rxd f5i rx b5i rx b5i o ro d6i rx a5i rx g4i rxd g4i rx g4i rx g4i rx g4i rx g4i rx g4i rx g4o // bar 12
                tdd rxd g4i rx g4i rx g4i rx g4i rx a4i rx a4i rx a4i rxd a4i ro a4i rxd a4i rx g4i rx g4tdd // bar 13
                o rx g4i rxd g4i rx g4i rx g4i rx a4i rxd a4i rx a4i rx a4i rxd a4i rx a4i rx g4i rx g4i rxd g4o // bar 14
                tdd rx g4i rx g4i rx g4i rx g4i rx g4i rx g4i rx g4i rx g4i rx g4i rxd g4i rx g4i rx g4i // bar 15
                rx g4i rx g4i rx g4i rx g4i rx g4i rxd g4i rx g4i rx g4i rx g4i rx g4i rx g4i rx g4i rxd g4t // bar 16
                t rx g4i rx g4i rx a4i rxd a4i rx a4i rx a4i rx a4i rxd a4i rx g4i rx g4i rx g4i rxd g4i ro // bar 17
                ro g4i rx g4i rx c4i rxd e4i ro as4i rxd f4i rx b4i rx as4i rx as3i rx ds4i rx e4i rxd ds4i rx as4t o // bar 18
                xd o ro f4i rx ds5i rx b5i rx a5i rx f6i rx a5i rxd as5i rx ds5i rx b5i rx as5i ro b5i rxd e6i rx d6x // bar 19
                td o rx a3i rx e4i rx b4e xd rx e3i rx b4i rx f3i rxd c4i rx a4e x rx c3i rxd a4td // bar 20
                x rx g3i rx d4i rxd g4e x ro d3i rxd g4i rx a3i rx e4i rx b4e xd ro e3i rxd a4i rx a3x // bar 21
                td rx e4i rx b4e xd ro e3i rx b4i rxd f3i rx c4i rx a4e x rx c3i rx a4i rx g3i ro // bar 22
                ro d4i rxd g4e xd ro d3i rx g4i rxd a3i rx e4i rx cs5e xd rx e3i rxd a4i rx d4i rx a3xd // bar 23
                t o rx ds4i rx e4i rxd as4i rx e4i rx g4i rx d4i rx gs4i rx as4i rx g4i rx ds4i rx e4i rx c4i rx // bar 24
                e4i rxd as4i rx e4i rx c4i rx g4i rx d4i rx gs4i rx as4i rx g4i rx ds4i rx e4i rxd ds4i ro e4tdd // bar 25
                o rxd as4i rx e4i rx d4i rx e4i rx c4i rxd e4i rx gs4i rx e4i rx c4i rxd fs4i rx d4i rx fs4i rx a4x // bar 26
                td rxd fs4i rx d4i rx fs4i rx d4i rx fs4i rx a4i rx fs4i rx ds4i rx e4i rx ds4i rxd e4i rx as4i // bar 27
                rx e4i rx d4i rx gs4i rx e4i rx gs4i rx b4i rx gs4i rxd e4i rx g4i rx d4i rx gs4i rx as4i rx g4t o // bar 28
                xd rx d4i rx g4i rx d4i rxd gs4i rx as4i rx g4i rx d4i rx g4i rxd d4i rx gs4i rx as4i rxd g4i rx // bar 29
                d4i rx g4i rx d4i rx gs4i rx as4i rxd g4i rx d4i ro a3i o ro e4i o ro b4e xd ro e3i o ro b4i // bar 30
                o ro f3i o ro c4i o ro a4e xd ro c3i o ro a4i o ro g3i rx d4i rx g4e x rx d3i rx g4i rx a3td // bar 31
                x rx e4i o ro b4e x rx e3i rxd a4i rx a3i o ro e4i rx b4e x rx e3i rx b4i rx f3i rx c4xd // bar 32
                t o rx a4e x rx c3i rx a4i rx g3i rx d4i rx g4e x rx d3i rx g4i rx a3i rx e4i o ro b4o // bar 33
                idd o xd ro e3i o rx a4i o ro a4i o ro f5i rx b5e x rx e4i rx b5i rx f4i rx b4i rxd as5tdd // bar 34
                i o x ro c4i rx a5i rxd g4i rx d5i rx g5e xd ro d4i rx g5i rx a4i rxd f5i rx b5idd // bar 35
                x x rx ds4i rx a5i rx a4i rx f5i rxd b5e x rx e4i rx b5i rx f4i rx b4i rx as5e x rx c4o // bar 36
                tdd rx a5i rx g4i rx d5i rx g5e xd ro d4i rx g5i rx a4i rx f5i rx b5e x rxd ds4i // bar 37
                rx a5i rx a4i rx f5i rx b5e xd rx e4i rx b5i rx f4i rxd b4i rx as5e xd ro c4i rx a5t // bar 38
                t rx g4i rxd d5i rx g5e x rxd d4i rx g5i rx a4i rx f5i rx b5e x rx ds4i rx a5i rx // bar 39
                a4i rx f5i rxd b5e xd ro e4i rx b5i rx f4i rx b4i rxd as5e x rx c4i rx a5i rx g4td // bar 40
                x rx d5i rx g5e xd ro g4i rxd gs5i rx cs5i rx f5i rx b5i rxd b5i o ro d6i rxd a5i rx g4i rx g4o // bar 41
                tdd rx g4i rxd g4i rx g4i rx g4i rx g4i rxd g4i rx g4i rx g4i rx g4i rx g4i rx a4i rx a4tdd // bar 42
                o rx a4i rxd a4i rx a4i rx a4i rx g4i rx g4i rx g4i rxd g4i rx g4i rx g4i rx a4i rx a4i rx a4xd // bar 43
                t o rxd a4i rx a4i ro a4i rxd g4i rx g4i rx g4i rx g4i rxd g4i rx g4i rx g4i rx g4i rxd g4i // bar 44
                rx g4i rx g4i rx g4i rx g4i rxd g4i rx g4i rx g4i rx g4i rxd g4i ro g4i rxd g4i rx g4i rx g4t // bar 45
                t rx g4i rx g4i rx g4i rx g4i rxd g4i rx g4i rx g4i rx g4i rx a4i rx a4i rx a4i rxd a4i rx // bar 46
                a4i rx a4i rx g4i rx g4i rx g4i rx g4i rxd g4i rx g4i rx c4i rx e4i rxd as4i rx f4i rx b4td // bar 47
                x rxd as4i rx as3i rx ds4i rx e4i rxd ds4i rx as4i rx f4i rx ds5i rx b5i rxd a5i ro f6i rx a5i rxd as5o // bar 48
                tdd rx ds5i rx b5i rx as5i rx b5i rx e6i o rx d6i x rqdd x // bar 49
                rhd // bar 50
                ),
        // arp2 (psg)
        ser!(
        voice_arp2() param!(velocity=93.13) a3tdd rxd c4i rx e4i rid g4i rx a3i rx g4i ro a3i rx c4i rid f4i rx f3i rx fs4i ro // bar 1
                ro b3i rx d4i rid d4i rx g3i rx d4i rx c4i rxd e4i rid g4i rx a3i rx e4i rx c4td // bar 2
                x rx e4i rid g4i o ro a3i rx g4i rx a3i rx c4i rid f4i o ro f3i rx fs4i rx b3i rx d4t // bar 3
                t rid d4i rx g3i rx d4i rx c4i rx e4i rid g4i rx a3i rxd param!(velocity=84.67) e4i o ro c5i o ro e5i rxd // bar 4
                ri x g5i rx a4i rx g5i rx as4i rx b4i rid f5i rxd f4i rx g5i rx as4i rx d5i ri o // bar 5
                rxd d5i rx g4i rx ds5i rx c5i rx e5i rid g5i rx a4i rx e5i rx c5i rx e5i rid o g5t // bar 6
                t rx a4i rx g5i rx as4i rx b4i rid f5i rx f4i rx g5i rx as4i rx d5i rid d5i rx g4x // bar 7
                td rx ds5i rxd c5i rx e5i rid g5i rx a4i rxd e5i rx c5i rx e5i rid o g5i rx a4tdd // bar 8
                o rx g5i rx as4i rx b4i rid f5i rx f4i rxd g5i rx as4i rx d5i rid d5i rx g4i rxd ds5xd // bar 9
                t o arp2i_seg_0() g5i rx a4i rx g5i rx // bar 10
                as4i rxd b4i rid f5i rx f4i rxd g5i rx as4i rx d5i rid o d5i rx g4i rx ds5i rx cs5t o // bar 11
                xd rxd f5i rx c6i rx e5i o ro as5i rx f5i rx e4i rxd e4i rx e4i rx e4i rx e4i rx e4i rx d4i rx d4o // bar 12
                tdd rxd d4i rx d4i rx d4i rx d4i rx c4i rx c4i rxd c4i rx c4i ro c4i rxd c4i rx e4i rx e4tdd // bar 13
                o rxd e4i rx e4i rx e4i rx e4i rxd c4i rx c4i rx c4i rx c4i rxd c4i rx c4i rx d4i rx d4i rxd d4o // bar 14
                tdd rx d4i rx d4i rx ds4i rx e4i rx e4i rx e4i rx e4i rx e4i rx e4i rxd d4i rx d4i rx d4i // bar 15
                rx d4i rx d4i rx ds4i rx e4i rx e4i rxd e4i rx e4i rx e4i rx e4i rx d4i rx d4i rx d4i rxd d4t // bar 16
                t rx d4i rx d4i rx c4i rxd c4i rx c4i rx c4i rx c4i rxd c4i rx e4i rx e4i rx e4i rxd e4i ro // bar 17
                ro e4i rx e4i rx a3i rxd c4i ro f4i rxd c4i rx a4i rx f4i rx d3i rx as3i rx d4i rxd as3i rx f4t o // bar 18
                xd o ro ds4i rx b4i rx e5i rx a5i rx as5i rxd gs5i rx f5i rx b4i rx b4i rx ds5i rx e5i rx fs5i rx g5x // bar 19
                td o rx c4i rx e4i rid o g4i rx a3i rx g4i rx a3i rxd c4i rid f4i rxd f3i rx fs4td // bar 20
                x rx b3i rx d4i rid o d4i rx g3i rx d4i rx c4i rx e4i rid g4i rx a3i rxd e4i rx c4x // bar 21
                td rx e4i rid g4i rx a3i rx g4i rxd a3i rx c4i rid f4i rx f3i rx fs4i rx b3i ro // bar 22
                ro d4i rid o d4i rx g3i rx d4i rxd cs4i rx e4i rid o g4i o ro a3i rxd e4i rx a3i rx f3xd // bar 23
                t o rx a3i rxd d4i rx f4i rx d4i rx d4i rx b3i rx d4i rx g4i rx d4i rx a3i rx d4i rx a3i rx // bar 24
                ro d4i rx f4i rx d4i rx a3i rx d4i rx b3i rx d4i rx g4i rx d4i rx b3i rxd d4i rx as3i ro d4tdd // bar 25
                o rxd f4i rx d4i rx as3i rx c4i rxd g3i rx c4i rx e4i rx c4i rx g3i rxd d4i rx a3i rx d4i rx fs4x // bar 26
                td rxd d4i rx a3i rx d4i rx a3i rx d4i rx fs4i rx d4i rx a3i rx d4i rx as3i rxd d4i rx f4i // bar 27
                rx d4i rx b3i rx e4i rx c4i rx e4i rx gs4i rx e4i rxd c4i rx d4i rx b3i rx d4i rx g4i rx d4t o // bar 28
                xd rx b3i rx d4i rx b3i rxd d4i rx g4i rx d4i rx b3i rx d4i rxd b3i rx d4i rx g4i rxd d4i rx // bar 29
                b3i rx d4i rx b3i rx d4i rx g4i rxd d4i rx b3i ro c4i o ro e4i o ri xd g4i o ro a3i o ro g4i // bar 30
                o ro a3i o ro c4i o ri xd f4i o ro f3i o ro fs4i o ro b3i rx d4i rid d4i rx g3i rx d4i rx c4td // bar 31
                x rx e4i o ri xd g4i rx a3i rxd e4i rx c4i o ro e4i rid g4i rx a3i rx g4i rx a3i rx c4xd // bar 32
                t o rid f4i rx f3i rx fs4i rx b3i rx d4i rid d4i rx g3i rx d4i rx c4i rx e4i rxd // bar 33
                ri o g4i o ro a3i rxd e4i o ro c5i o ro e5i rid g5i rx a4i rx g5i rx as4i rxd b4i ri o // bar 34
                rx f5i rx f4i rxd g5i rx as4i rx d5i rid d5i rx g4i rx ds5i rx c5i rxd e5i rid g5t // bar 35
                t rx a4i rx e5i rx c5i rxd e5i rid g5i rx a4i rx g5i rx as4i rx b4i rid f5i rx f4o // bar 36
                tdd rx g5i rx as4i rx d5i rid d5i rx g4i rx ds5i rx c5i rx e5i rid o g5i rx a4i // bar 37
                rx e5i rx c5i rx e5i rid g5i rxd a4i rx g5i rx as4i rxd b4i rid f5i rx f4i rx g5t // bar 38
                t rxd as4i rx d5i rid d5i rxd g4i rx ds5i arp2i_seg_0() // bar 39
                g5i rx a4i rx g5i rx as4i rx b4i rid o f5i rx f4i rx g5i rx as4td // bar 40
                x rx d5i rid d5i rx g4i rxd ds5i rx cs5i rx f5i rx c6i rxd e5i o ro as5i rxd f5i rx e4i rx e4o // bar 41
                tdd rx e4i rxd e4i rx e4i rx e4i rx d4i rxd d4i rx d4i rx d4i rx d4i rx d4i rx c4i rx c4tdd // bar 42
                o rx c4i rxd c4i rx c4i rx c4i rx e4i rx e4i rx e4i rxd e4i rx e4i rx e4i rx c4i rx c4i rx c4xd // bar 43
                t o rxd c4i rx c4i ro c4i rxd d4i rx d4i rx d4i rxd d4i rx d4i rx ds4i rx e4i rx e4i rxd e4i // bar 44
                rx e4i rx e4i rx e4i rxd d4i rx d4i rx d4i rx d4i rx d4i rxd ds4i ro e4i rxd e4i rx e4i rx e4t // bar 45
                t rx e4i rx e4i rx d4i rx d4i rxd d4i rx d4i rx d4i rx d4i rx c4i rx c4i rxd c4i rx c4i rx // bar 46
                c4i rx c4i rx e4i rx e4i rx e4i rx e4i rxd e4i rx e4i rx a3i rx c4i rxd f4i rx c4i rx a4td // bar 47
                x rxd f4i rx d3i rx as3i rx d4i rxd as3i rx f4i rx ds4i rx b4i rx e5i rxd a5i ro as5i rxd gs5i rx f5o // bar 48
                tdd rx b4i rx b4i rx ds5i rx e5i rx fs5i rxd a5i x rqdd x // bar 49
                rhd // bar 50
                ),
        // harmony3 (ym)
        ser!(
        rw // bar 1
                rw // bar 2
                voice_harmony3() e5i ro a4i rx b4i o ro c5i o ro a4i rx e4i o ro e5i o ro a4i rx b4i o ro c5i o ro a4i rx e4i rx e5i ro // bar 3
                ro a4i rx b4i rx c5i rx a4i rx e4i rx e5i rx a4i rx b4i rx c5i rx a4i o rx e4i o ro voice_harmony3_2() e3i o ro f3td // bar 4
                x o ro g3i o ro a3i o rx b3i o ro c4i o ro b3o rx c4x rx b3q ro a3e x rxd g3e x rx d3x // bar 5
                idd x rx f3e xd ro e3q tdd ro e3e xd ro e3i o ro f3i o ro g3i o ro // bar 6
                ro a3i rx b3i rx c4i rx d4edd rx e4i ro c4e xd rx b3e xd ro d3e o // bar 7
                x ro g3e xd ro a3q tdd rx a3xd rxd c3i rx d3i rx e3i rx f3i rxd g3i rx a3tdd // bar 8
                o rx g3o rx a3x rx g3q ro f3e x rx d3e xd rx b2e xd ro d3e xd ro c3t // bar 9
                edd td rxd c3e x rx c3i rx d3i rx e3i rx f3i rxd g3i rx a3i rx c4i x // bar 10
                e td tdd rx b3id rx a3o rx b3x ro a3q tdd rx e3e x rxd e3idd o // bar 11
                e o tdd rx voice_harmony3_3() c4i rx d4i rx e4edd o rx f4i rx g4e xd ro d4e t o // bar 12
                i xd tdd rx c4i rx b3i rx c4edd rx d4i rxd e4e x rx e4e xd ro b3tdd // bar 13
                i o xd rx a3i rx g3i rx a3edd o rx b3i rx c4e x rxd d4e x rx c4id // bar 14
                t x rxd d4e x rx e4hd xd // bar 15
                rx c4i rx d4i rx e4edd rxd f4i rx g4e x rx d4q tdd rx c4t // bar 16
                t rx b3i rx c4edd o rx d4i rx e4e xd rx e4e x rx b3e x rxd a3i ro // bar 17
                ro g3i rx a3edd rx b3i rxd c4e xd ro e4e xd ro d4e xd rx c4idd o // bar 18
                o x rx e4qdd x rxd e4qdd x rx voice_harmony3_4() a2x // bar 19
                td o rx c3i rx e3i rx b3i rxd g3i rx e3i rx f2i rx a2i rxd e3i rx a3i rx e3i rx c3i rxd g2td // bar 20
                x rx d3i rx g3i rx a3i rxd g3i ro d3i rx a2i rxd e3i rx b3i rx a3i rx b3i rx e3i rx a2i rxd c3x // bar 21
                td rx e3i rx b3i rx g3i rx e3i rx f2i rx a2i rxd e3i rx a3i rx e3i rx c3i rx g2i rx d3i ro // bar 22
                ro g3i rxd a3i rx g3i rx d3i rx a2i rxd e3i rx a3i rx cs4i rxd e4i rx cs4i rx voice_harmony3_5() d4i rxd d4i rt o // bar 23
                rtdd a3i rx d4i rxd a3i rx e4i rx e4i rid c4i rx e4i rx c4i rx f4i rx f4i rid // bar 24
                d4i rxd f4i rx d4i rx e4i rx e4i rid c4i rx e4i rx c4i rx d4i rx d4i rid as3tdd // bar 25
                o o rx d4i rx as3i rx e4i rx e4i rid o c4i rx e4i rx c4i rx fs4i rx fs4i rid o a3i rx fs4x // bar 26
                td rxd a3i rx fs4i rx fs4i rid a3i rx d4i rx a3i rx d4i rx d4i rid o as3i rx d4i // bar 27
                rx as3i rx e4i rx e4i rid c4i rx e4i rx c4i rxd d4i rx d4i rid b3i rx d4i rx b3t o // bar 28
                xd rx d4i o ro d4i rid o b3i rx d4i rx b3i rx b2e xd rx a2e xd ro b2e x rxd // bar 29
                g2e x rx f2e x rx g2e x rxd c3h xd // bar 30
                w // bar 31
                edd o idd re o voice_harmony3() e5i rx a4i rx b4i rx c5i rx a4i rx e4i rx e5i // bar 32
                rx a4i rx b4i rx c5i rx a4i rx e4i rx e5i rx a4i rx b4i rx c5i rx a4i rx e4i o ro e5i rx a4td // bar 33
                x rx b4i o ro c5i o ro a4i o ro e4i o rx voice_harmony3_2() e3i rx f3i o ro g3i rx a3i rx b3i rx c4i rx b3o rx c4x rx b3e // bar 34
                e ro a3e xd ro g3e xd rx d3e xd ro f3e xd ro e3eddd // bar 35
                x tdd rx e3e x rx e3i rx f3i rxd g3i rx a3i ro b3i o ro c4i rx d4edd o rx e4td // bar 36
                x o ro c4e xd ro b3e xd ro d3e xd ro g3e x rx a3q td rx a3xd ro // bar 37
                rx c3i ro d3i rxd e3i rx f3i rx g3i rxd a3i rx g3x ro a3o rxd g3q rx f3e xd ro d3t // bar 38
                id x rxd b2e x rx d3e x rxd c3q td rx c3e x rx c3i rx // bar 39
                d3i rx e3i rxd f3i rx g3i rx a3i rx c4q td rxd b3i xd rx a3x ro b3o rxd a3e // bar 40
                e td rx e3e xd rx e3q tdd rx voice_harmony3_3() c4i o ro d4i rxd e4e t o // bar 41
                tdd o ro f4i rxd g4e x rx d4q tdd rx c4i rx b3i rx c4ed xd // bar 42
                o o ro d4i rxd e4e xd ro e4e xd ro b3e xd rx a3i rx g3i rx a3edd rx b3xd // bar 43
                t o rx c4e x rx d4e xd rx c4e xd ro d4e xd rx e4q tdd // bar 44
                qdd o t rx c4i rx d4i rxd e4edd rx f4i rx g4t // bar 45
                id xd ro d4q tdd rx c4i rx b3i rx c4edd rx d4i rxd e4i x // bar 46
                td x rx e4e x rx b3e xd ro a3i rx g3i rxd a3edd o rx b3i rx c4e // bar 47
                xd ro e4e xd rx d4e x rxd c4e x rx e4qdd xd rx e4o // bar 48
                qd tdd xd rh td // bar 49
                rhd // bar 50
                ),
        // harmony2 (ym)
        ser!(
        rw // bar 1
                rhdd voice_harmony2() e5i rx a4td // bar 2
                x rx b4i rx c5i rx a4i rx e4i o ro e5i rx a4i rx b4i rx c5i rx a4i rx e4i rx e5i rx a4i rx b4t // bar 3
                t rx c5i rx a4i rx e4i rx e5i rx a4i rx b4i rx c5i rx a4i rx e4i rx voice_harmony2_2() a3i o rx c4i o ro e4i rx b4o // bar 4
                tdd rxd g4i rx e4i rx f3i rx a3i rx e4i rx a4i rx e4i rx c4i rxd g3i rx d4i rx g4i rx a4tdd // bar 5
                o rx g4i rx d4i rx a3i rx e4i rx b4i rx a4i rx b4i rx e4i rx a3i rx c4i rx e4i rxd b4i rx g4t // bar 6
                t rx e4i rx f3i rx a3i rx e4i rx a4i rx e4i rx c4i rx g3i rx d4i rx g4i rx a4i rx g4i rx d4x // bar 7
                td rx a3i rx e4i rxd b4i rx a4i rx b4i o ro e4i o ro e3q tdd rxd c3e o // bar 8
                o rx a3e x rx f3e xd ro a3e xd ro g3e xd rx d3e xd ro f3e xd ro e3t // bar 9
                edd td rxd e3e x rx e3q tdd ro c3e xd rx a3i x // bar 10
                e x o rx b3i o ro c4e xd ro b3e xd rx d3e x rx g3e x rxd a3idd o // bar 11
                q i o xd rx c4edd o rx d4i rx e4e xd ro b3e t o // bar 12
                i xd tdd rx a3i rx g3i rx a3edd rx b3i rxd c4e x rx b3e xd ro g3tdd // bar 13
                i o xd rx f3x ro g3o rxd f3o rx e3i o ro f3edd o rx g3i o ro a3e x rxd b3e x rx a3id // bar 14
                t x rxd b3e x rx c4qdd xd ro g3q tdd // bar 15
                rx g3e x rx c4edd rxd d4i rx e4e x rx b3q tdd ro a3t o // bar 16
                xd rxd g3i o ro a3edd o rx b3i o ro c4e xd rx b3e x rx g3e x rxd f3o rx g3x ro f3o rx // bar 17
                ro e3i rx f3edd rx g3i rxd a3e xd ro as3e xd ro f3i rx a3xd rx f3xd rxd d3idd o // bar 18
                o x rx e2e xd ro a2t o ro b2t o rx a2t o rx fs2e x rxd gs2i xd gs2o gs2td rx fs2e x ro gs2i x gs2o gs2tdd rxd e3x // bar 19
                td o rx f3i o ro g3i rx a3i o rx b3i rx c4i o ro b3x rx c4x ro b3q o ro a3e x rxd g3td // bar 20
                i x x rx d3e x rxd f3e x ro e3q tdd rx e3e xd ro e3i rxd f3x // bar 21
                td rx g3i rx a3i rx b3i rx c4i rx d4edd rxd e4i rx c4e x rx b3e xd // bar 22
                ro d3e xd rx g3e xd ro a3qddd rx f4i re // bar 23
                rtdd f4i rid o f4i rx g4i re td g4i rid g4i rx a4i rx a4i o ri xd // bar 24
                a4i rxd a4e xd ro g4i rx g4i o ri xd g4i rx g4e x rx f4i rx f4i rid f4tdd // bar 25
                o o rx f4e xd ro g4i o ro g4i o rid g4i o ro g4e xd ro a4i o ro a4i o rid a4i rx a4x // bar 26
                idd x rxd a4i rx a4i rid a4i rx a4e x rx f4i rx f4i o ri xd f4i rxd f4i // bar 27
                i xd ro g4i rx g4i rid g4i rx g4e x rx g4i rx g4i rid o g4i rx g4idd o // bar 28
                o x rx g4i o ro g4i o ri xd g4i rxd g4e xd ro g4e xd rx f4e xd ro g4e x rxd // bar 29
                d4e x rx c4e x rx d4e x rxd e4h xd // bar 30
                w // bar 31
                edd o idd rx voice_harmony2() e5i o ro a4i o ro b4i rx c5i rx a4i rx e4i rx e5i rx a4i rx b4xd // bar 32
                t o rx c5i rx a4i rx e4i rx e5i rx a4i rx b4i rx c5i rx a4i rx e4i rx e5i rx a4i rx b4i rx c5o // bar 33
                tdd rx a4i rx e4i o ro voice_harmony2_2() a3i o rx c4i o ro e4i rx b4i rx g4i rx e4i rx f3i rx a3i rx e4i rxd a4tdd // bar 34
                o ro e4i rx c4i o ro g3i rx d4i rxd g4i rx a4i rx g4i rx d4i rx a3i rx e4i rx b4i rxd a4i rx b4t // bar 35
                t rx e4i rx a3i rx c4i rx e4i rxd b4i rx g4i rx e4i ro f3i rx a3i rxd e4i rx a4i rx e4i rx c4o // bar 36
                tdd rx g3i rx d4i rx g4i rx a4i rx g4i rx d4i rx a3i rx e4i rx b4i rx a4i rx b4i rx e4i ro // bar 37
                rx e3q tdd ro c3e xd rx a3e xd ro f3e xd rx a3e xd ro g3t // bar 38
                id x rxd d3e x rx f3e x rxd e3q td rx e3e xd ro e3i x // bar 39
                e td tdd rx c3e xd ro a3edd rxd b3i rx c4e x rx b3e // bar 40
                x rx d3e x rx g3e xd rx a3qddd ro c4e td // bar 41
                td o rx d4i rxd e4e x rx b3q tdd rx a3i rx g3i rx a3ed xd // bar 42
                o o ro b3i rxd c4e xd ro b3e xd ro g3e xd ro f3o rxd g3o rx f3x rx e3i rx f3edd rx g3xd // bar 43
                t o rx a3e x rx b3e xd rx a3e xd ro b3e xd rx c4q tdd // bar 44
                e t o ro g3q tdd rx g3e x rxd c4edd rx d4i rx e4t // bar 45
                id xd ro b3q tdd rx a3i rx g3i rx a3edd rx b3i rxd c4i x // bar 46
                td x rx b3e x rx g3e xd ro f3x rx g3o rx f3o rx e3i o rx f3edd o rx g3i o ro a3e // bar 47
                xd ro as3e xd rx f3i rx a3xd rx f3xd rxd d3e x rx e2e x rx a2t o rx b2t o rx a2t o ro fs2e xd rx gs2o // bar 48
                tdd xd gs2o gs2tdd ro fs2e xd ro gs2i xd gs2o gs2tdd ro voice_harmony2_3() a3i o rqddd // bar 49
                rhd // bar 50
                ),
        // harmony (ym)
        ser!(
        voice_harmony() a2w // bar 1
                h eddd o as2xd b2t c3i rx a3e // bar 2
                hdd e // bar 3
                idd rx b3i rx c4i rx a3qdd x rxd c3i o ro e3i o ro a3i rxd // bar 4
                re t a3i rx c3i rx f3i rx a3i re td a3i rxd c3i rx d3i rx g3i ri o // bar 5
                rid o g3i rx d3i rx e3i rx a3i re td a3i rx c3i rx e3i rx a3i re o // bar 6
                rtd a3i rx c3i rx f3i rx a3i re td a3i rx c3i rx d3i rx g3i re td g3x // bar 7
                td rx d3i rx e3i rxd a3i rid voice_harmony_2() a3o gs3x g3x fs3o f3x e3x ds3o d3x cs3x c3o b2x rxd a2e x rx b2i rx a2i rxd e3e o // bar 8
                o rx f2e x rx a2i rx f2i rx c3e x rxd g2e xd ro b2i rx g2i rx d3e x rx a2t // bar 9
                id x rxd b2i rx a2i rx e3e x rx a2e x rx b2i rx a2i rxd e3e xd ro f2i x // bar 10
                td xd rx a2i rx f2i rx c3e xd rx g2e xd ro b2i rx g2i rxd d3e x rx a2idd o // bar 11
                o x rxd cs3i rx a2i rx e2i o ro f2i rx c3e xd rid c3e xd ro c3i rx b2e xd rx // bar 12
                ri x b2e x rx b2i rx a2e x rid a2e x rx a2i rxd g2e xd ri // bar 13
                rxd g2e xd rx g2i rx f2edd o rx g2i rx a2e x rxd b2e x rx a2id // bar 14
                t x rxd b2e x rx c3edd o ro d3i rx c3e xd ro b2e xd rx a2e x // bar 15
                o ro b2e x rx c3e x rid o c3e x rx c3i rx b2e x rid b2idd o // bar 16
                o xd rx b2i rx a2e xd rid a2e xd ro a2i rxd g2e x rid g2e x rx // bar 17
                ro g2i rx f2edd rx g2i rxd a2e xd ro as2e x rx c3e xd rx d3idd o // bar 18
                o x rx e3edd o ro a2i rx b2e x rxd gs2e x rx fs2e x ro gs2e x rxd a2x // bar 19
                td o rx a2i rid a2e xd rx a2i rx f2i rx f2i rid o f2e x rx f2i rxd g2td // bar 20
                x rx g2i rid g2e x rx g2i rx a2i rxd a2i rid a2e xd ro a2i rx a2i o rx a2x // bar 21
                td rid a2e xd ro a2i rx f2i rx f2i rid o f2e x rx f2i rx g2i rx g2i ro // bar 22
                rid g2e xd ro g2i rx a2i rxd a2i rid a2i rxd b2i o ro cs3i rx d3i rxd d3i rt o // bar 23
                rtdd d3i rx d3e x rxd e3i rx e3i rid e3i rx e3e x rx f3i rx f3i rid // bar 24
                f3i rxd f3e xd ro g3i rx g3i rid g3i rx g3e x rx as3i rx c4i rxd d4i ro c4tdd // bar 25
                o rxd as3i rx d4i rx c4i rx g3i rx e3i rxd f3i rx g3i rx e3i rx d3i rx e3i rxd fs3i rx e3i rx fs3x // bar 26
                td rxd a3i rx d4q td rx a3e x rx as3e x rx f3e xd rx d3i // bar 27
                i xd ro g3e x rx e3e xd ro c3e x rxd g3e x rx d3e x rx b2idd o // bar 28
                o x rx d3e xd ro g3e xd rx d3e xd ro g3e xd rx a3e xd ro d4e x rxd // bar 29
                b3e x rx a3e x rx g3e x rxd a3h xd // bar 30
                w // bar 31
                edd o idd rx voice_harmony() a3h e xd // bar 32
                q id o eddd rx b3i rx c4i rx a3eddd o // bar 33
                ed o xd rx c3i o ro e3i o ro a3i re td a3i rx c3i rx f3i rx a3i ri x // bar 34
                rid a3i rx c3i rxd d3i rx g3i re td g3i rx d3i rx e3i rx a3i re o // bar 35
                rtd a3i rx c3i rx e3i rx a3i re tdd a3i ro c3i rx f3i rxd a3i re td a3o // bar 36
                tdd rx c3i rx d3i rx g3i re td g3i rx d3i rx e3i rx a3i rid o voice_harmony_2() a3o gs3o g3x fs3x f3o e3x ds3o d3o cs3x c3x b2o rx // bar 37
                rx a2e x rx b2i rx a2i rx e3e xd rx f2e xd ro a2i rxd f2i rx c3e xd ro g2t // bar 38
                id x rxd b2i rx g2i rx d3e x rxd a2e x rx b2i rx a2i rx e3e x rx a2i x // bar 39
                td xd ro b2i rxd a2i rx e3e xd ro f2e xd ro a2i rxd f2i rx c3e x rx g2e // bar 40
                x rx b2i rx g2i rx d3e xd rx a2e xd ro cs3i rx a2i rxd e2i o ro f2i rxd c3e xd rx // bar 41
                ri o c3e x rxd c3i rx b2e x rid o b2e x rx b2i rx a2e xd ri // bar 42
                rxd a2e xd rx a2i rx g2e x rid g2e xd rx g2i rx f2edd rx g2xd // bar 43
                t o rx a2e x rx b2e xd rx a2e xd ro b2e x rxd c3edd o ro d3i ro // bar 44
                rx c3e xd ro b2e x rxd a2e x rx b2e x rxd c3e x rid c3idd // bar 45
                x xd ro c3i rx b2e xd rid b2e xd ro b2i rx a2e x rid a2e x rxd // bar 46
                a2i rx g2e x rid g2e xd rx g2i rx f2edd o rx g2i rx a2e // bar 47
                xd rx as2e xd ro c3e x rxd d3e x rx e3edd rxd a2i ro b2e xd rx gs2o // bar 48
                idd o x rx fs2e xd ro gs2e x rxd voice_harmony_3() a2i o rqdd xd // bar 49
                rhd // bar 50
                ),
        // bass (ym)
        ser!(
        voice_bass() comment!("approx timbre") e0o f0xd fs0xd g0xd gs0x repeat!(1) a0t as0t b0t c1x repeat!(1) cs1t o d1t o ds1xd ds1x e1td f1td fs1o fs1t o g1tdd gs1t c_1i o cs_1o d_1o e_1o f_1o fs_1o gs_1o a_1o as_1o b_1o c0o cs0t d0o ds0o e0x f0o fs0x g0o gs0o repeat!(1) a0x as0x b0x c1x cs1xd d1x ds1x ds1o e1xd f1x // bar 1
                o fs1xd g1xd gs1x repeat!(1) a1xd as1t b1t c2x c2xd cs2t o d2t o ds2xd ds2x e2td f2td fs2o fs2t o g2td gs2t c_1t o d_1o f_1o g_1o a_1o b_1o cs0o ds0o e0o fs0o g0o gs0o a0o as0o b0o c1o cs1o d1o ds1x e1o f1x fs1o g1x gs1o repeat!(1) a1x as1x b1x c2o re xd // bar 2
                rw // bar 3
                rhd voice_bass_2() c3i o rx param!(op2_tl=11, op4_tl=11) c3i o ro c3i o ro voice_bass_3() d3o // bar 4
                tdd rx voice_bass_4() c3i rxd c3i o ro param!(op2_tl=7, op4_tl=7) c3i o ro param!(op2_tl=11, op4_tl=11) c3i o ro voice_bass_5() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_4() c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rxd param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx c3i rx voice_bass_3() d3tdd // bar 5
                o rx voice_bass_4() c3i rx c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx voice_bass_5() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx param!(op2_tl=7, op4_tl=7) c3i o ro param!(op2_tl=11, op4_tl=11) c3i rx c3i rxd voice_bass_3() d3i rx voice_bass_4() c3t // bar 6
                t rx c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx voice_bass_5() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx param!(op2_tl=7, op4_tl=7) c3i o ro param!(op2_tl=11, op4_tl=11) c3i o ro c3i o ro voice_bass_6() c3i rx voice_bass_4() c3i rx c3x // bar 7
                td o ro param!(op2_tl=7, op4_tl=7) c3i o ro param!(op2_tl=11, op4_tl=11) c3i o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro e2o ds2o d2o cs2o b1o as1o gs1o g1o f1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd voice_bass_4() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o // bar 8
                d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_4() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx c3i rx voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_4() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o // bar 9
                gs1o fs1o f1o ds1o d1o rx voice_bass_4() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx c3i rx voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_4() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_6() c3i o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro // bar 10
                voice_bass_4() c3i rxd voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_4() c3i o ro c3i o ro voice_bass_6() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_4() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() c3i rxd voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_4() c3t o // bar 11
                xd rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_4() c3i ro c3i o rx voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_7() c3o // bar 12
                b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() c3i ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o // bar 13
                d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_6() c3i rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_6() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_8() b1o // bar 14
                as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rxd voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o // bar 15
                d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o // bar 16
                f2o ds2o d2o c2o rx b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_6() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_6() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o ro // bar 17
                ro b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_6() c3i rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i o rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro c3o b2o as2o a2o gs2o // bar 18
                fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o g2o f2o e2o d2o cs2o rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx c3i rx voice_bass_7() b2o as2o a2o gs2o g2o f2o e2o d2o cs2o as2o a2o gs2o g2o f2o e2o d2o cs2o b1o ro gs2o g2o fs2o f2o ds2o d2o cs2o b1o a1o rx voice_bass_8() b1o as1o // bar 19
                a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_4() c3i o ro c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_4() c3i rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_4() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_4() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd b1o as1o a1o gs1o fs1o f1o // bar 20
                ds1o d1o rx voice_bass_4() c3i rx c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_4() c3i ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_4() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_4() c3x // bar 21
                td rx c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_4() c3i rxd voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_4() c3i o // bar 22
                ro c3i o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_4() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_4() c3i o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_4() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd voice_bass_6() c1xd // bar 23
                t o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() g1i rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_6() f1i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro param!(op2_tl=11, op4_tl=15) f2o e2o ds2o d2o cs2o b1o as1o gs1o g1o ro d2o cs2o c2o b1o as1o gs1o g1o fs1o e1o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c1i rx // bar 24
                voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() g1i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() f1i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro param!(op2_tl=11, op4_tl=15) f2o e2o ds2o d2o cs2o b1o as1o gs1o g1o ro d2o cs2o c2o b1o as1o gs1o g1o fs1o e1o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c1i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o // bar 25
                d2o c2o rx voice_bass_6() g1i o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_6() f1i o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro param!(op2_tl=11, op4_tl=15) e2o ds2o d2o cs2o b1o as1o gs1o g1o f1o ro cs2o c2o b1o as1o gs1o g1o fs1o e1o d1o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_6() c1i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() g1x // bar 26
                td rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd voice_bass_4() c1i rx c1i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx b2o as2o a2o gs2o fs2o f2o ds2o d2o rx param!(op2_tl=11, op4_tl=15) f2o e2o ds2o d2o cs2o b1o as1o gs1o g1o ro d2o cs2o c2o b1o as1o gs1o g1o fs1o e1o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c1i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() g1i // bar 27
                rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() f1i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro param!(op2_tl=11, op4_tl=15) f2o e2o ds2o d2o cs2o b1o as1o gs1o g1o ro d2o cs2o c2o b1o as1o gs1o g1o fs1o e1o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_6() c1i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() g1i rx voice_bass_8() b1o as1o a1o gs1o fs1o // bar 28
                f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_6() f1i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx param!(op2_tl=11, op4_tl=15) e2o ds2o d2o cs2o b1o as1o gs1o g1o f1o ro cs2o c2o b1o as1o gs1o g1o fs1o e1o d1o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_6() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_6() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd // bar 29
                voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx b2o as2o a2o gs2o fs2o f2o ds2o d2o rx cs2o c2o b1o as1o gs1o g1o fs1o e1o rid b1o as1o a1o gs1o fs1o f1o ds1o d1o rx param!(op2_tl=11, op4_tl=15) b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx param!(op2_tl=15, op4_tl=19) c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro param!(op2_tl=19, op4_tl=23) c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro param!(op2_tl=23, op4_tl=27) c3o b2o as2o a2o gs2o fs2o f2o ds2o // bar 30
                d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro param!(op2_tl=27, op4_tl=31) c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro param!(op2_tl=31, op4_tl=35) c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rh id o // bar 31
                rw // bar 32
                rw // bar 33
                redd o voice_bass_2() c3i o rx param!(op2_tl=11, op4_tl=11) c3i o ro c3i rx voice_bass_3() d3i rx voice_bass_4() c3i rx c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx voice_bass_5() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i // bar 34
                rx param!(op2_tl=7, op4_tl=7) c3i o ro param!(op2_tl=11, op4_tl=11) c3i o ro param!(op2_tl=7, op4_tl=7) c3i o ro param!(op2_tl=11, op4_tl=11) c3i o rx c3i o ro voice_bass_3() d3i rx voice_bass_4() c3i rx c3i o ro param!(op2_tl=7, op4_tl=7) c3i o ro param!(op2_tl=11, op4_tl=11) c3i o ro voice_bass_5() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_4() c3i rx param!(op2_tl=7, op4_tl=7) c3t // bar 35
                t rx param!(op2_tl=11, op4_tl=11) c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx c3i rxd voice_bass_3() d3i rx voice_bass_4() c3i rx c3i ro param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rxd voice_bass_5() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3o // bar 36
                tdd rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx c3i rx voice_bass_6() c3i rx voice_bass_4() c3i rx c3i rx param!(op2_tl=7, op4_tl=7) c3i rx param!(op2_tl=11, op4_tl=11) c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro f2o e2o ds2o d2o cs2o b1o as1o gs1o g1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o // bar 37
                rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o voice_bass_4() c3i o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_6() c3i o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_4() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_4() c3i rx c3i o ro voice_bass_6() c3i rx voice_bass_8() b1o as1o a1o gs1o // bar 38
                fs1o f1o ds1o d1o c1o ro voice_bass_4() c3i rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_4() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx c3i rx voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro // bar 39
                voice_bass_4() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_4() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_4() c3i rx c3i rxd voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_4() c3td // bar 40
                x rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_6() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_4() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_4() c3i o rx c3i o ro voice_bass_6() c3i o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_6() c3i rx voice_bass_8() b1o // bar 41
                as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() c3i rxd voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o // bar 42
                ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rxd voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o // bar 43
                a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rxd voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_6() c3i rxd voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_6() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o // bar 44
                rx voice_bass_6() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_6() c3i rxd voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3t // bar 45
                t rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_6() c3i rx // bar 46
                voice_bass_7() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_8() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_6() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_6() c3i o ro voice_bass_7() b2o as2o a2o gs2o fs2o f2o // bar 47
                ds2o d2o c2o ro voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_6() c3i rx voice_bass_8() b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd voice_bass_7() b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_6() c3i rx voice_bass_7() b2o as2o a2o gs2o g2o f2o e2o d2o cs2o rx voice_bass_8() c2o // bar 48
                b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_6() c3i rx c3i o ro voice_bass_7() b2o as2o a2o gs2o g2o f2o e2o d2o cs2o ro b2o as2o a2o gs2o g2o f2o e2o d2o cs2o ro a2o gs2o g2o fs2o f2o ds2o d2o cs2o b1o ro param!(op2_tl=8, op4_tl=12) c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o rqddd // bar 49
                rhd // bar 50
                ),
        // drums (psg)
        ser!(
        rw // bar 1
                rw // bar 2
                rw // bar 3
                rw // bar 4
                rw // bar 5
                rw // bar 6
                rw // bar 7
                rw // bar 8
                rw // bar 9
                rw // bar 10
                rw // bar 11
                rw // bar 12
                rw // bar 13
                rw // bar 14
                rw // bar 15
                rw // bar 16
                rw // bar 17
                rw // bar 18
                rw // bar 19
                rw // bar 20
                rw // bar 21
                rw // bar 22
                rw // bar 23
                rw // bar 24
                rw // bar 25
                rw // bar 26
                rw // bar 27
                rw // bar 28
                rw // bar 29
                rw // bar 30
                rw // bar 31
                rw // bar 32
                rw // bar 33
                rw // bar 34
                rw // bar 35
                rw // bar 36
                rw // bar 37
                rw // bar 38
                rw // bar 39
                rw // bar 40
                rw // bar 41
                rw // bar 42
                rw // bar 43
                rw // bar 44
                rw // bar 45
                rw // bar 46
                rw // bar 47
                rw // bar 48
                rqddd voice_drums() param!(velocity=101.6) c4h t // bar 49
                qddd q t // bar 50
                )
    )
}

#[rustfmt::skip]
pub fn loop_alisia_stage1() -> Note {
    par!(
        param!(tempo=116.955444),
        // melody (psg)
        ser!(
        rw // bar 1
                rw // bar 2
                rw // bar 3
                rw // bar 4
                rw // bar 5
                rqd t voice_melody() param!(velocity=8.47) d6o param!(velocity=50.8) e6i o ds4i o ro a5i o ro a4i o ro f5i o ro f4i o ro b4i o rx b5t o // bar 6
                i xd xd ro f4i o ro e5i o ro as4i rx b4i rx g4i rx d5i rxd b5e x rx as4i ro g5i o rx g4i rx ds5x // bar 7
                td o ro e4i o ro as4i o rx a5e xd ro b4i o ro e5xd rqdd xd param!(velocity=8.47) g4o param!(velocity=50.8) gs4td // bar 8
                x ro ds4i rxd a5i rx a4i rx f5i rxd f4i ro b4i o ro b5e xd rx f4i o ro e5i o ro as4i o ro b4i o ro g4xd // bar 9
                t o o ro d5i o ro b5e xd ro as4i o ro g5i rxd g4i rx d5i rx a4i rx f5i rx b5e x rx ds4i o ro // bar 10
                ro a5xd rhddd t // bar 11
                rw // bar 12
                rw // bar 13
                rw // bar 14
                rw // bar 15
                rw // bar 16
                rw // bar 17
                rw // bar 18
                rw // bar 19
                rw // bar 20
                reddd // bar 21
                ),
        // lead (ym)
        ser!(
        ri o voice_lead_5() f4i rx param!(op4_tl=21) f4i rx param!(op4_tl=17) g4i ro param!(op4_tl=21) g4i o rx param!(op4_tl=17) a4i o ro param!(op4_tl=21) a4i o ro param!(op4_tl=17) b4t ro c5t ro b4qd o // bar 1
                tdd xd ro g4t o gs4t o a4i o ro g4q i fs4t o f4t o e4t o ds4xd rxd c4t o cs4t o d4i o ro f4e // bar 2
                id o ro e4edd o ro c4t o cs4t o d4i o rx e4i o ro f4i rx param!(op4_tl=21) f4i rx param!(op4_tl=17) g4i rx param!(op4_tl=21) g4i rx param!(op4_tl=17) a4i rx param!(op4_tl=21) a4xd // bar 3
                t o rx param!(op4_tl=17) b4xd rxd c5xd ro d5t o ds5t o e5q i ds5t o d5t ro c5t o cs5t o d5i o ro c5edd o rx // bar 4
                cs5t o d5ed ro b4e xd rx b4edd rx a4edd o ro gs4xd rx g4xd rx fs4t rx f4o // bar 5
                x rid voice_lead_6() e3i o rx f3i o ro g3i o ro a3i o ro b3i rx c4i o ro b3qd t o // bar 6
                i xd t o rx a3e x rid g3i o ro a3i o ro b3i o rx b3i o ro c4i o ro d4i o ro c4e xd // bar 7
                q id o td rx e3e xd ri xd e3i rxd f3i rx g3i rx a3i o ro b3t // bar 8
                id xd rx c4i o ro d4t ro ds4xd rx e4qd o rx d4e xd ro c4e t // bar 9
                i rxd d4edd rx b3e x rx a3q i g3t o f3t o e3t o d3t rx voice_lead_7() c4td // bar 10
                x o ro d4i o ro e4i o rx f4e xd ro g4h e xd // bar 11
                t o o rx d4e xd ro c4i rx b3i o ro c4i o rx d4e x rx e4edd o ro f4e x ro // bar 12
                ro e4edd rx b3edd rx a3t b3o rx a3o rx g3i o ro a3edd o ro b3e o // bar 13
                i xd o rx c4e xd ro d4edd rx e4edd o rx f4e x rx g4id o // bar 14
                qd xd idd o rx d4qdd x rx // bar 15
                ro c4i rx d4i rx e4i rx f4e x rx g4h id o // bar 16
                i xd o ro d4e xd rx c4i o ro b3i o ro c4i rx d4e x rx e4edd rxd f4id o // bar 17
                xd x rx e4edd o ro b3edd o rx a3xd b3x ro a3x rx g3i o ro a3edd rx b3i x // bar 18
                e x rx c4e x rx d4edd rx e4edd o ro f4e xd rx e4tdd // bar 19
                qd t o ro e4idd d4t c4xd b3qd t o // bar 20
                i xd e ro voice_lead_5() e4x // bar 21
                ),
        // lead2 (ym)
        ser!(
        voice_lead2_5() f4i o ro param!(op4_tl=13) f4i rx param!(op4_tl=9) g4i rx param!(op4_tl=13) g4i rx param!(op4_tl=9) a4i rxd param!(op4_tl=13) a4i rx param!(op4_tl=9) b4xd rx c5xd rx b4qdd o // bar 1
                o ro g4t o gs4t o a4i rxd g4q i fs4t o f4t o e4t o ds4xd rx c4t o cs4t o d4i rx f4ed o // bar 2
                xd rxd e4edd rx c4t o cs4t o d4i o ro e4i o ro f4i o ro param!(op4_tl=13) f4i rx param!(op4_tl=9) g4i rx param!(op4_tl=13) g4i o rx param!(op4_tl=9) a4i o ro param!(op4_tl=13) a4i rx param!(op4_tl=9) b4o // bar 3
                x rx c5xd rx d5t o ds5t o e5q i ds5t o d5xd rx c5t o cs5t o d5i rx c5edd rx cs5t o d5t // bar 4
                e t ro b4e xd rx b4edd o ro a4edd rxd gs4xd rx g4xd rx fs4xd rx f4xd rtdd // bar 5
                rtd voice_lead2_6() e3i rx f3i rxd g3i rx a3i ro b3i o rx c4i rx b3qddd o // bar 6
                xd td ro a3e xd ri xd g3i rx a3i rx b3i rx b3i rxd c4i rx d4i rx c4ed xd // bar 7
                q t o t o rx e3e xd rid e3i o ro f3i o ro g3i o ro a3i rxd b3id // bar 8
                t x rx c4i rx d4xd rxd ds4xd rx e4qd o ro d4e xd rx c4edd // bar 9
                o ro d4edd o ro b3e x rx a3q i o g3t o f3t o e3t o d3xd rx voice_lead2_7() c4i o ro d4t o // bar 10
                xd o ro e4i o rx f4e xd ro g4h ed o rx d4o // bar 11
                idd o x rxd c4i rx b3i rx c4i rx d4e x rx e4edd o ro f4e xd ro e4i // bar 12
                e t o ro b3edd o rx a3xd b3x ro a3o rxd g3i rx a3edd rx b3ed o // bar 13
                xd rx c4e xd ro d4edd o rx e4edd o ro f4e xd ro g4e td // bar 14
                q i x idd o rx d4qdd xd rx c4i // bar 15
                o ro d4i rx e4i o ro f4e xd ro g4h e td // bar 16
                x o rx d4e xd ro c4i o ro b3i o ro c4i o ro d4e xd rx e4edd o ro f4e x rx e4x // bar 17
                ed x rxd b3edd o ro a3xd b3o rxd a3o rx g3i rx a3edd o ro b3e xd // bar 18
                i o o ro c4e xd ro d4edd o rx e4edd rx f4e x rx e4e // bar 19
                q i xd rx e4idd d4t c4o rx b3qddd x // bar 20
                x idd o rx voice_lead2_5() e4i o rx // bar 21
                ),
        // arp (psg)
        ser!(
        voice_arp() param!(velocity=67.73) a3e td rq tdd param!(velocity=84.67) a3q i x red o // bar 1
                rxd f3e t c3h t o rx d3e t g3idd // bar 2
                x t o re xd a3i x e3i x g3i x e3q re xd a3e t o // bar 3
                ri o a3eddd o redd o f3e t d3q tdd // bar 4
                i o rx g3eddd o param!(velocity=67.73) e3qddd o param!(velocity=84.67) a3xd a4xd param!(velocity=76.2) gs4t f4xd e4xd param!(velocity=67.73) d4t ro param!(velocity=84.67) c4t // bar 5
                t rx a3i rxd e4i rx b4e x rx e3i rx a4i rx a3i rx e4i rx f3i rx c4i rxd as4e xd ro // bar 6
                f3i rx e4i rx a3i rx c4i rx g3i rx d4i rxd as4e x rx b3i rx g4i rx g3i rx d4i rx e3tdd // bar 7
                o rx b3i rx gs4e xd rx b3i rx e4i rx g3i rxd b3i rx a3i rx e4i rx b4e x rxd e3i rx a4x // bar 8
                td rxd a3i rx e4i rx f3i rx c4i rx as4e xd ro f3i rxd e4i rx a3i rx c4i rx g3i rx d4i // bar 9
                rx as4e xd ro b3i rx g4i o ro g3i rxd d4i rx a3i rx e4i rx b4e x rx e3i rx a4i o ro a3t o // bar 10
                xd rxd e4i o ro c4e xd ro c4e xd rx c4i rx c4i rx c4xd rx c4xd rx c4i rx b3e x rxd b3i x // bar 11
                td x rx b3i rx b3i rx b3xd rx b3xd rxd b3i rx a3e x ro a3e x rx a3i rx a3i rx a3xd rx a3xd rxd a3tdd // bar 12
                o rx g3e x rx g3e x rx g3i rx g3i rx g3xd rx g3xd rx g3i rx f3e x rx f3e x rx f3t o // bar 13
                xd rx f3i rx f3xd rx f3xd rx f3i rxd g3e xd ro g3e x rx g3i rxd g3i rx g3xd rx g3xd rx g3i rx g3i xd // bar 14
                t o x rx g3e x rxd g3i rx g3i rx g3xd rx g3xd rx g3i rx g3e x rx g3e x rx g3i rx g3i // bar 15
                rx g3xd rx g3t rx g3i rx c4e x rx c4e x rx c4i rx c4i rx c4xd rx c4xd rx c4i rx b3e x rx b3t o // bar 16
                i xd xd ro b3i rx b3i rx b3xd rx b3xd rxd b3i rx a3e xd ro a3e x rx a3i rx a3i rx a3xd rx a3xd rxd a3o // bar 17
                tdd rx g3e x rx g3e x rx g3i rxd g3i ro g3xd rx g3xd rx g3i rx f3e x rx f3e x ro // bar 18
                ro f3i rxd f3i rx f3xd ro f3t rx f3i rx f3e x rx f3e x rx f3i rx f3i rx f3xd rx f3xd rxd f3i rx gs3t o // bar 19
                i xd x rx gs3e xd ro gs3i rx gs3i rxd gs3xd rx gs3xd rx gs3i rx e3e xd ro e3e x rx e3i rxd e3o // bar 20
                tdd rx e3xd rx e3xd rxd e3i x // bar 21
                ),
        // arp2 (psg)
        ser!(
        voice_arp2() param!(velocity=67.73) e4e td rq tdd param!(velocity=84.67) e4q i x red o // bar 1
                rxd c4e t g3h t o rx a3e t ds4idd // bar 2
                x t o re xd e4i x b3i x d4i x b3q re xd e4e t o // bar 3
                ri o e4eddd o redd o c4e t a3q tdd // bar 4
                i o rxd d4eddd o param!(velocity=67.73) b3qddd o param!(velocity=84.67) e4xd f5xd param!(velocity=76.2) d5t ds5xd cs5xd param!(velocity=67.73) c5t param!(velocity=84.67) b4t // bar 5
                t rx b4i rxd gs5i rx d6e x rx a4i rx c6i rx e5i rx g5i rx a4i rx fs5i rxd c6e xd ro // bar 6
                a4i rx as5i rx b4i rx g5i rx as4i rxd g5i rx d6e x rx ds5i rx b5i rx as4i rx g5i rx g4tdd // bar 7
                o rx f5i rx as5e xd rx e5i rx a5i rx b4i rxd e5i rx b4i rx gs5i rxd d6e x rx a4i rx c6x // bar 8
                td rxd e5i rx g5i rx a4i rxd fs5i ro c6e xd ro a4i rxd as5i rx b4i rx g5i rx as4i rx g5i // bar 9
                rx d6e xd ro ds5i rx b5i rx as4i rxd gs5i rx b4i rx gs5i rx d6e x rx a4i rx c6i o ro e5t o // bar 10
                xd rxd gs5i rx e5e xd rx e5e x rx e5i rx e5i rx e5xd rx e5xd rx e5i rx d5e x rxd d5i x // bar 11
                td x rx d5i rx d5i rx d5xd rx d5xd rxd ds5i rx c5e x ro c5e x rx c5i rx c5i rx c5xd rx c5xd rxd c5tdd // bar 12
                o rx b4e x rx b4e x rx b4i rx b4i rx b4xd rx b4xd rx as4i rx a4e x rx a4e x rx a4t o // bar 13
                xd rx a4i rx a4xd rx a4xd rxd as4i rx b4e xd ro b4e x rx b4i rxd b4i rx b4xd rx b4xd rx b4i rx c5i xd // bar 14
                t o x rx c5e x rxd c5i rx c5i rx c5xd rx c5xd rx c5i rx b4e x rx b4e x rx b4i rx b4i // bar 15
                rx b4xd rx b4t rx b4i rx e5e x rx e5e x rx e5i rx e5i rx e5xd rx e5xd rx e5i rx d5e x rx d5t o // bar 16
                i xd xd ro d5i rx d5i rx d5xd rx d5xd rxd ds5i rx c5e x rx c5e x rx c5i rx c5i rx c5xd rx c5xd rxd c5o // bar 17
                tdd rx b4e x rx b4e x rx b4i rxd b4i ro b4xd rx b4xd rx as4i rx a4e x rx a4e x ro // bar 18
                ro a4i rxd a4i rx a4xd rx a4t ro a4i rx as4e x rx as4e x rx as4i rx as4i rx as4xd rx as4xd rxd b4i rx b4t o // bar 19
                i xd x rx b4e xd ro b4i rx b4i rxd b4xd rx b4xd rx as4i rx gs4e xd ro gs4e x rx gs4i rxd gs4o // bar 20
                tdd rx gs4xd rx gs4xd rxd a4i x // bar 21
                ),
        // harmony3 (ym)
        ser!(
        rt o voice_harmony3_6() e4i o ro f4i rx param!(op4_tl=24) f4i rx param!(op4_tl=20) g4i rxd param!(op4_tl=24) g4i rx param!(op4_tl=20) a4i rx param!(op4_tl=24) a4i rx param!(op4_tl=20) b4xd rx c5xd rx b4q i x // bar 1
                idd x rx g4t o gs4t o a4i o ro g4q i fs4t o f4t o e4t o ds4xd rx c4t o cs4t o d4i rxd f4i o // bar 2
                e xd rx e4edd o ro c4t o cs4t o d4i o ro e4i rx f4i rx param!(op4_tl=24) f4i o rx param!(op4_tl=20) g4i o ro param!(op4_tl=24) g4i rx param!(op4_tl=20) a4td // bar 3
                x rx param!(op4_tl=24) a4i o ro param!(op4_tl=20) b4xd rx c5xd rx d5t o ds5t o e5q i ds5t o d5xd rx c5t o cs5t o d5i rx c5ed // bar 4
                t o ro cs5t o d5ed rx b4e xd ro b4edd o rx a4edd rx gs4xd rxd voice_harmony3_7() e5t // bar 5
                t rx a4i rxd param!(op2_tl=38, op4_tl=23) a4i rx param!(op2_tl=34, op4_tl=19) b4i rx param!(op2_tl=38, op4_tl=23) b4i rx param!(op2_tl=34, op4_tl=19) c5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4xd rx a4t ro param!(op2_tl=34, op4_tl=19) e5i rx a4i o ro param!(op2_tl=38, op4_tl=23) a4i rx param!(op2_tl=34, op4_tl=19) b4i rxd param!(op2_tl=38, op4_tl=23) b4i rx // bar 6
                param!(op2_tl=34, op4_tl=19) c5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4xd rx a4t ro param!(op2_tl=34, op4_tl=19) e5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4i rx param!(op2_tl=34, op4_tl=19) b4i rxd param!(op2_tl=38, op4_tl=23) b4i rx param!(op2_tl=34, op4_tl=19) c5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4xd ro a4xd rxd param!(op2_tl=34, op4_tl=19) e5i rx a4tdd // bar 7
                o rx param!(op2_tl=38, op4_tl=23) a4i rx param!(op2_tl=34, op4_tl=19) b4i rxd param!(op2_tl=38, op4_tl=23) b4i rx param!(op2_tl=34, op4_tl=19) c5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4xd rx a4xd rxd param!(op2_tl=34, op4_tl=19) e5i o ro a4i rx param!(op2_tl=38, op4_tl=23) a4i rx param!(op2_tl=34, op4_tl=19) b4i rxd param!(op2_tl=38, op4_tl=23) b4i rx param!(op2_tl=34, op4_tl=19) c5i rx a4x // bar 8
                td rx param!(op2_tl=38, op4_tl=23) a4xd rxd a4xd rx param!(op2_tl=34, op4_tl=19) e5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4i rx param!(op2_tl=34, op4_tl=19) b4i rx param!(op2_tl=38, op4_tl=23) b4i rx param!(op2_tl=34, op4_tl=19) c5i rxd a4i rx param!(op2_tl=38, op4_tl=23) a4xd rx a4t ro param!(op2_tl=34, op4_tl=19) e5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4i // bar 9
                rx param!(op2_tl=34, op4_tl=19) b4i rx param!(op2_tl=38, op4_tl=23) b4i rx param!(op2_tl=34, op4_tl=19) c5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4t ro a4xd rxd param!(op2_tl=34, op4_tl=19) e5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4i rx param!(op2_tl=34, op4_tl=19) b4i rx param!(op2_tl=38, op4_tl=23) b4i rx param!(op2_tl=34, op4_tl=19) c5i rx a4i rx param!(op2_tl=38, op4_tl=23) a4xd rx // bar 10
                a4xd redd voice_harmony3_8() e4i o ro g4i o rx c5e xd ro g4e xd ro e4i o ro g3i o ro d4i rxd g4t o // bar 11
                xd rx b4e x rx d5i rx g4i rx d4i rx a3i o ro e4i o ro a4i o ro c5e xd ro e5i o ro a4i o ro e4xd // bar 12
                t o o ro g3i o ro d4i o rx g4i o ro d5e x rx b4i rx g4i rx e4i rx f3i rx c4i rx f4i rx a4i x // bar 13
                td xd ro e5i rx c5i o ro a4i o ro g4i o rx g3i o ro b3i o ro d4e xd ro b4i rx g4i o rx d4i rx voice_harmony3_6() d4t o ds4o // bar 14
                xd o e4q i ds4t o d4t o cs4xd rxd d4h t // bar 15
                i x o red x voice_harmony3_8() e4i o ro g4i o rx c5e x rx g4e x rx e4i rx g3i rx d4i rx // bar 16
                g4i o ro b4e xd ro d5i rx g4i o rx d4i o ro a3i o ro e4i rx a4i o ro c5e xd ro e5i rx a4tdd // bar 17
                o rxd e4i rx g3i rx d4i rx g4i rx d5e x rx b4i o ro g4i o ro e4i o ro f3i o ro c4i o ro f4i o ro a4t // bar 18
                id xd ro c5e xd rx e5i o ro f5i o ro d5e x rx as4e x rx f4i rx d4xd rxd voice_harmony3_9() e2idd o // bar 19
                q i o xd re o g5xd rx gs5xd rx a5xd rx as5xd rx b5q idd o // bar 20
                i o xd rx a5xd rid o // bar 21
                ),
        // harmony2 (ym)
        ser!(
        voice_harmony2_3() a3i rxd param!(op4_tl=17) a3i rx param!(op4_tl=21) a3i rq param!(op4_tl=9) a3i rx f3i rx f3i rx param!(op4_tl=17) f3i rx param!(op4_tl=21) f3i re xd // bar 1
                rxd param!(op4_tl=9) c3e x rx d3i o ro d3i rx param!(op4_tl=17) d3i rx param!(op4_tl=21) d3i rq param!(op4_tl=9) d3i rxd g3i rx a3i rx param!(op4_tl=17) a3t // bar 2
                t rx param!(op4_tl=21) a3i rid param!(op4_tl=9) e3i rx g3i rx e3i rx a3i rx a3i rxd param!(op4_tl=17) a3i rx param!(op4_tl=21) a3i rid param!(op4_tl=9) a3i rx param!(op4_tl=17) a3i rxd // bar 3
                ri o param!(op4_tl=9) f3i rx f3i rx param!(op4_tl=17) f3i rx param!(op4_tl=21) f3i re td param!(op4_tl=9) d3e x rx g3i rx g3i rx param!(op4_tl=17) g3i rx param!(op4_tl=21) g3i ro // bar 4
                ri xd param!(op4_tl=9) e3i rx param!(op4_tl=17) e3i rxd param!(op4_tl=9) e3i rx e4edd o ro e4eddd o ds4t o d4t o cs4t o c4xd rxd voice_harmony2_4() e4t // bar 5
                qdd ro c4e xd ro a4qd o rx // bar 6
                f4i o ro a4e x rx g4edd rx a4edd rxd b4e x rx c5e o // bar 7
                q i xd ro e4e xd rx c5edd o ro a4edd o rx e4x // bar 8
                idd x rxd g4qdd xd rx f4e x rx e4edd // bar 9
                o ro d4edd o ro b3e x rxd a3q td rq t o // bar 10
                re voice_harmony2_5() e4i o ro g4i rx c5e xd rx g4e xd ro e4i rx g3i rx d4i rx g4i rx b4i xd // bar 11
                t o x rxd d5i rx g4i rx d4i rx a3i rxd e4i ro a4i rx c5e x rx e5i rx a4i rx e4i o ro g3i // bar 12
                rx d4i rxd g4i rx d5e x rx b4i rx g4i rx e4i rx f3i rx c4i rx f4i rx a4e x rx e5t o // bar 13
                xd rx c5i rx a4i rx g4i rx g3i rxd b3i rx d4e xd ro b4i rx g4i rx d4i rxd voice_harmony2_6() d4t o ds4t o e4i xd // bar 14
                e t o i ds4t o d4t o cs4t o c4t o b3t o as3xd rxd g3h t // bar 15
                i x o rid voice_harmony2_5() e4i rx g4i rx c5e x rx g4e x rx e4i rx g3i rx d4i rx g4i rx b4t o // bar 16
                i xd xd ro d5i rx g4i rx d4i rx a3i rxd e4i rx a4i rx c5e xd ro e5i rx a4i rx e4i rxd g3o // bar 17
                tdd rx d4i rx g4i rx d5e x rx b4i rxd g4i ro e4i rx f3i rx c4i rx f4i rx a4e x ro // bar 18
                ro c5e xd ro e5i o rx f5i rx d5e x rx as4e x rx f4i rx d4i rx as3i rxd voice_harmony2_6() gs2idd o // bar 19
                q i o xd ro g5xd rx gs5xd rxd a5xd rx as5t ro b5qdd xd ro a5x // bar 20
                o rxd g5xd rx f5xd rx e5xd rx param!(op4_tl=9) a3i o rx // bar 21
                ),
        // harmony (ym)
        ser!(
        voice_harmony_3() a2i rqd t o a2i rx f2i rx f2i rq tdd // bar 1
                rxd c2e x rx d2i rx d2i rqd t o d2i rx g2i rx a2i rtd // bar 2
                red x e2i rx g2i rx e2i rx a2i rxd a2i rq a2i rid o // bar 3
                ri o f2i rx f2i rq i x d2e x rxd g2i ro g2i re t o // bar 4
                ri xd e2i rid o e2i rx param!(op4_tl=34) a2i o ro param!(op4_tl=30) a2i rx param!(op4_tl=26) a2i rx param!(op4_tl=22) a2i rxd param!(op4_tl=18) a2i rx a2i rx a3xd g3t f3t e3xd d3xd c3x rx a2t // bar 5
                t rx a2i rxd a2i rx a2i rx a2i rx a2i o ro a2i o ro a2xd rx a2t ro f2i rx f2i o ro f2i rxd f2i rx f2i rx // bar 6
                f2i rx g2i o ro a2i rx g2i rx g2i rx g2i rx g2i rxd g2i rx g2i rx d2i rx d2i rx e2i rx e2tdd // bar 7
                o rx e2i rx e2i rxd g2i rx e2i rx b2i rx g2i rxd a2i o ro a2i rx a2i rx a2i rxd a2i rx a2i rx a2x // bar 8
                td rxd a2xd rx a2xd rx f2i rx f2i rx f2i rx f2i rx f2i rx f2i rxd f2i o ro f2i rx g2i o ro g2i rx g2i // bar 9
                rx g2i o ro g2i rx g2i rx g2i o ro g2i rxd a2i rx a2i rx c3i rx a2i rx e3i rx a2i rx b2i o ro d3t o // bar 10
                xd rxd c3i o ro c3e xd ro c3e xd rx c3i rx c3i rx c3xd rx c3xd rx b2i rx b2e x rxd b2i x // bar 11
                td x rx b2i rx b2i rx b2xd rx b2xd rx a2i rxd a2e x ro a2e x rx a2i rx a2i rx a2t ro a2xd rxd g2tdd // bar 12
                o rx g2e x rx g2xd rx g2t ro b2i rx g2i rx d3i rx b2i rx f2i rx f2e x rx f2e x rx g2t o // bar 13
                xd rx a2i rx f2i rx g2i rxd g2e xd ro g2i rx b2i rx g2i rx d3i rxd g2i rx c3i rx c3i xd // bar 14
                t o x rx c3e x rx d3i rxd c3i rx f2i rx g2i rx gs2e x rx a2e x rx as2i rx b2i // bar 15
                rx d3i rx c3i o rx c3e x rx c3e x rx c3i rx c3i rx c3xd rx c3xd rx b2i rx b2e x rx b2t o // bar 16
                i xd xd ro b2i rx b2i rx b2xd rx b2xd rxd a2i rx a2e xd ro a2e xd ro a2i rx a2i rx a2xd rx a2xd rxd g2o // bar 17
                tdd rx g2e x rx g2xd rx g2xd rx b2i rx g2i rxd d3i ro b2i rx f2i rx f2e x rx f2e x ro // bar 18
                ro f2i rxd g2i ro a2i rxd as2i rx as2e x rx as2e x rx as2i rx f2i rx d2i rxd e2i rx e2t o // bar 19
                i xd x rx e2e xd ro e2i rx e2i rxd e2xd rx e2xd rx gs2i rx gs2i rx gs2i rx gs2i rx b2i rx gs2i rxd d3o // bar 20
                tdd rx e2i rxd a2i rx // bar 21
                ),
        // bass (ym)
        ser!(
        voice_bass_9() comment!("approx timbre") c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o re td voice_bass_10() e3i rx voice_bass_11() e3i rx voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rq td // bar 1
                rxd b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o as0o gs0o f0o d0o as_1o f_1o c_1x gs2o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o re td voice_bass_10() e3i rx voice_bass_11() e3i rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd voice_bass_13() fs2o f2o e2o ds2o cs2o ro param!(op2_tl=12, op4_tl=25) g2o fs2o f2o e2o ds2o cs2o ro param!(op2_tl=16, op4_tl=29) fs2o // bar 2
                f2o e2o ds2o cs2o rx param!(op2_tl=20, op4_tl=33) fs2o f2o e2o ds2o cs2o ro param!(op2_tl=24, op4_tl=37) g2o fs2o f2o e2o ds2o cs2o ro param!(op2_tl=28, op4_tl=41) fs2o f2o e2o ds2o cs2o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o re td voice_bass_10() e3i rx voice_bass_13() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3xd // bar 3
                t o xd ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rq i o voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_13() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx b2o as2o a2o gs2o fs2o rx param!(op2_tl=12, op4_tl=25) b2o as2o // bar 4
                a2o gs2o fs2o rx param!(op2_tl=16, op4_tl=29) c3o b2o as2o a2o gs2o fs2o voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rxd voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_14() b2x as2o a2o gs2o ro c3o b2x as2o a2o gs2o rx c3o b2x as2o a2o gs2o fs2x f2o e2o ds2o rx g2o fs2x f2o e2o ds2o ro g2o fs2x f2o e2o ds2o voice_bass_9() b2o as2o a2o gs2o fs2o rx c3o b2o as2o a2o gs2o fs2o ro c3o b2o as2o a2o gs2o rx voice_bass_13() b2o as2o a2o gs2o fs2o ro c3o b2o as2o a2o gs2o fs2o ro b2o as2o a2o gs2o fs2o rx voice_bass_12() b1x as1o a1o gs1o // bar 5
                fs1o f1o ds1o d1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rxd voice_bass_11() e3i rx voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o as0o gs0o f0o d0o as_1o f_1o c_1x gs2o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_11() e3i rx // bar 6
                voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx b1o as1o a1o gs1o fs1o f1o ds1o // bar 7
                d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro b2o as2o a2o gs2o ro b2o as2o a2o gs2o ro voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rxd voice_bass_11() e3i rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o as0o gs0o f0o // bar 8
                d0o as_1o f_1o c_1x gs2o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o rxd voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_11() e3i rx voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o // bar 9
                d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o ro c3o b2o as2o a2o ro c3o b2o as2o a2o ro c3o b2o as2o a2o ro voice_bass_14() gs2o repeat!(1) g2o fs2o ro gs2o repeat!(1) g2o fs2o ro e2x ds2o d2o ro // bar 10
                e2x ds2o d2o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rxd voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd // bar 11
                voice_bass_11() e3i rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o // bar 12
                d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_11() e3i rx voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro b1o as1o a1o gs1o fs1o // bar 13
                f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_11() e3i rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rxd voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() b2o // bar 14
                as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o rxd voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o ro c3o b2o as2o a2o ro c3o b2o as2o a2o ro c3o b2o as2o a2o ro voice_bass_14() a2o gs2o repeat!(1) g2o ro a2o gs2o repeat!(1) // bar 15
                g2o ro a2o gs2o repeat!(1) g2o ro a2o gs2o repeat!(1) g2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o voice_bass_12() b1o as1o a1o gs1o fs1o f1o // bar 16
                ds1o d1o c1o rx voice_bass_11() e3i rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() b1o as1o // bar 17
                a1o gs1o fs1o f1o ds1o d1o rxd b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_11() e3i rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o rx voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o // bar 18
                ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o rx voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o ro voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_11() e3i rx voice_bass_12() c2o b1o as1o a1o gs1o fs1o f1o ds1o d1o ro voice_bass_9() c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o as1o gs1o f1o d1o as0o f0o a_1o c_1o gs3o ro c3o b2o as2o a2o gs2o fs2o f2o ds2o d2o rx voice_bass_12() c2o b1o as1o a1o gs1o // bar 19
                fs1o f1o ds1o d1o voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o rx voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o ro voice_bass_9() b2o as2o a2o gs2o fs2o f2o ds2o d2o c2o ro voice_bass_12() b1o as1o a1o gs1o fs1o f1o ds1o d1o c1o rx voice_bass_9() b2o as2o a2o gs2o ro b2o as2o a2o gs2o ro b2o as2o a2o gs2o ro b2o as2o a2o gs2o ro voice_bass_14() gs2o repeat!(1) g2o fs2o ro gs2o repeat!(1) g2o fs2o ro gs2o repeat!(1) g2o fs2o ro gs2o repeat!(1) g2o fs2o ro e2x ds2o d2o ro e2x ds2o d2o ro e2x ds2o d2o ro e2x ds2o d2o ro voice_bass_9() b2o as2o // bar 20
                a2o rxd b2o as2o a2o rx b2o as2o a2o rx b2o as2o a2o rx b2x as2o a2o gs2o fs2o f2o ds2o d2o rx // bar 21
                ),
        // drums (psg)
        ser!(
        rw // bar 1
                rhd i voice_drums() param!(velocity=101.6) c4ed // bar 2
                w // bar 3
                h i e t rq t // bar 4
                rq i x param!(velocity=84.67) c4h e td // bar 5
                q i x i o rh id o // bar 6
                rw // bar 7
                rh o param!(velocity=101.6) c4qdd rtdd // bar 8
                rw // bar 9
                rw // bar 10
                rtd c4qdd xd rqddd xd // bar 11
                rw // bar 12
                rw // bar 13
                rw // bar 14
                rhddd c4i // bar 15
                qdd e rqdd // bar 16
                rw // bar 17
                rw // bar 18
                rhdd o c4idd o // bar 19
                q id o o rh e o c4o // bar 20
                edd o o // bar 21
                )
    )
}

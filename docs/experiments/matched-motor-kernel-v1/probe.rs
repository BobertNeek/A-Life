//! Detached scalar diagnostics only; no actor, world, sampling or extra settling.
use matched_motor_kernel::{Control, Kernel, Modulation, SETPOINTS};
fn main() {
    println!("case,microstep,recovery,search,fear,rest_delta,approach_object_delta,ingest_positive_delta,inspect_object_delta");
    for (name, h, f, atp, fear) in [
        ("setpoints", 0.35, 0.35, 0.5, 0.25),
        ("hunger", 0.9, 0.35, 0.5, 0.25),
        ("fatigue", 0.35, 0.9, 0.5, 0.25),
        ("conflicting_hunger_fatigue", 0.9, 0.9, 0.5, 0.25),
        ("low_atp", 0.35, 0.35, 0.1, 0.25),
        ("high_atp", 0.35, 0.35, 0.9, 0.25),
        ("fear_fatigue", 0.35, 0.9, 0.5, 0.9),
        ("hunger_low_atp", 0.9, 0.35, 0.1, 0.25),
    ] {
        let mut drives = SETPOINTS;
        drives[0] = h;
        drives[1] = f;
        drives[2] = fear;
        drives[6] = atp;
        let mut k = Kernel::new(Control::Full);
        for step in 1..=96 {
            k.step(drives, Modulation::default()).unwrap();
            if [1, 2, 3, 4, 6, 12, 24, 48, 96].contains(&step) {
                let mut intrinsic = [0.; 54];
                intrinsic[23] = 1.;
                let mut object = [0.; 54];
                object[36] = 1.;
                let mut positive = [0.; 54];
                positive[37] = 1.;
                let [r, v, e] = k.controller();
                let rest = k.candidate_delta(1, &intrinsic).unwrap();
                let approach = k.candidate_delta(3, &object).unwrap();
                let ingest = k.candidate_delta(6, &positive).unwrap();
                let inspect = k.candidate_delta(2, &object).unwrap();
                println!("{name},{step},{r:.9},{v:.9},{e:.9},{rest:.9},{approach:.9},{ingest:.9},{inspect:.9}");
            }
        }
    }
}

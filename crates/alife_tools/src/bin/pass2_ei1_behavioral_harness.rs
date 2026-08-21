use std::process;

fn main() {
    if let Err(error) =
        alife_tools::pass2_ei1_behavioral::run(std::env::args().skip(1).collect())
    {
        eprintln!("pass2_ei1_behavioral_harness: {error}");
        process::exit(1);
    }
}

//! Host entry point; live packet handling is restricted to the disposable lab.

#[cfg(target_os = "linux")]
mod lab;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    #[cfg(target_os = "linux")]
    if let [command, run_id] = args.as_slice()
        && command == "lab"
    {
        if let Err(error) = lab::run(run_id) {
            eprintln!("lab failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    if !args.is_empty() && args != ["--help"] {
        eprintln!("Usage: irongated [--help | lab RUN_ID]");
        std::process::exit(2);
    }
    println!(
        "IronGate {} — development scaffold",
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "Normal deployment is not implemented. The lab command requires the owned QEMU guest harness."
    );
    println!("See docs/roadmap.md for milestones and acceptance criteria.");
}

//! Host entry point for the IronGate gateway scaffold.

fn main() {
    println!(
        "IronGate {} — development scaffold",
        env!("CARGO_PKG_VERSION")
    );
    println!("Networking and access enforcement are not implemented.");
    println!("See docs/roadmap.md for milestones and acceptance criteria.");
}

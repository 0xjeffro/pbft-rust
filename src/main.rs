use clap::{Arg, Command};
use pbft_rust::network::launcher;
fn main() {
    let matches = Command::new("pbft-rust")
        .version("1.0.0")
        .author("Jeffro")
        .about("A sample implementation of the Practical Byzantine Fault Tolerance (PBFT) consensus algorithm in Rust. 🦀")
        .arg(
            Arg::new("f")
                .short('f')
                .long("number of faulty nodes")
                .value_parser(clap::value_parser!(u32))
                .help("Sets the number of faulty nodes"),
        )
        .arg(
            Arg::new("n")
                .short('n')
                .long("number of nodes")
                .value_parser(clap::value_parser!(u32))
                .help("Sets the number of nodes"),
        )
        .get_matches();

    let f = *matches.get_one::<u32>("f").unwrap_or(&1);
    let n = *matches.get_one::<u32>("n").unwrap_or(&4);

    if n < f {
        panic!("The number of nodes must be greater than the number of faulty nodes.");
    }

    println!("f: {}", f);
    println!("n: {}", n);

    launcher::launch(n, f).unwrap();

}
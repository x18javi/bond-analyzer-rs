pub fn main() {
    if let Err(e) = bond_analyzer::get_args().and_then(bond_analyzer::run) {
        println!("{}", e);
        std::process::exit(1);
    }
}

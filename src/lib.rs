mod args;
mod bond;
use bond::Bond;
use clap::Parser;
use thiserror::Error;

type CliResult<T> = Result<T, String>;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("could not parse value for [ {0} ]")]
    BadParseError(String),
}

pub fn get_args() -> CliResult<args::BondCli> {
    let matches = args::BondCli::parse();
    Ok(matches)
}

pub fn run(bond: args::BondCli) -> CliResult<()> {
    let bond = Bond::new(bond).map_err(|e| e.to_string())?;
    let cashflows_table = bond.cashflows_table();
    let analysis_table = bond.analysis_table();

    println!("{}\n{}", cashflows_table, analysis_table);

    Ok(())
}

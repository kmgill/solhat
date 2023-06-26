mod subs;
use anyhow::Result;
use colored::Colorize;
use subs::runnable::RunnableSubcommand;
use subs::*;

#[macro_use]
extern crate stump;

extern crate wild;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "solhat")]
#[clap(about = "Solar Hydrogen Alpha Processing", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: SolHat,

    #[clap(long, short, help = "Verbose output")]
    verbose: bool,
}

#[derive(Subcommand)]
enum SolHat {
    Process(process::Process),
    PreProcess(preprocess::PreProcess),
    Mean(mean::Mean),
    Median(median::Median),
    ThreshTest(threshtest::ThreshTest),
    SerInfo(serinfo::SerInfo),
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let t1 = std::time::Instant::now();

    stump::set_min_log_level(stump::LogEntryLevel::WARN);
    info!("Initialized logging"); // INFO, which means that this won't be seen
                                  // unless the user overrides via environment
                                  // variable.

    let args = Cli::parse_from(wild::args());

    if args.verbose {
        stump::set_verbose(true);
    }

    if let Err(why) = match args.command {
        SolHat::Process(args) => args.run().await,
        SolHat::PreProcess(args) => args.run().await,
        SolHat::Mean(args) => args.run().await,
        SolHat::Median(args) => args.run().await,
        SolHat::ThreshTest(args) => args.run().await,
        SolHat::SerInfo(args) => args.run().await,
    } {
        error!("{}", "Unhandled program error:".red());
        error!("{}", why);
    };
    info!("Runtime: {}s", t1.elapsed().as_secs_f64());
    Ok(())
}

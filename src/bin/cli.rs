use std::{collections::HashMap, path::PathBuf};

use arr::Arr;
use clap::{Args, Parser, Subcommand};

use clap_verbosity_flag::Verbosity;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    verbose: Verbosity,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an AtomicRedTeam Test
    Run(Run),
    /// Helpful developer utilities
    #[command(subcommand)]
    Utils(Utils),
    /// Run the clean up for a Test
    Cleanup(Cleanup),
}

#[derive(Args)]
struct Run {
    /// Technique number
    technique: String,

    /// Test number
    #[arg(default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..))]
    test_number: u8,

    /// Set a variable
    #[arg(long = "set-var", short = 's', value_parser = parse_vars, value_name = "VARIABLE=VALUE")]
    vars: Vec<(String, String)>,

    /// Path to ART yaml files
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Cleanup immediately after running the test
    #[arg(short, long, default_value_t = false)]
    cleanup: bool,
}

#[derive(Args)]
struct Cleanup {
    /// Technique number
    technique: String,

    /// Test number
    #[arg(default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..))]
    test_number: u8,

    /// Set a variable
    #[arg(long = "set-var", short = 's', value_parser = parse_vars, value_name = "VARIABLE=VALUE")]
    vars: Vec<(String, String)>,

    /// Path to ART yaml files
    #[arg(short, long, default_value = ".")]
    path: PathBuf,
}

#[derive(Subcommand)]
enum Utils {
    /// Tests the parser on all YAML files in the path
    ParseAll(TestPath),
    /// List the number of times each executor is used
    ListExecutors(TestPath),
    // /// Run all tests for this OS
    // RunAll(TestPath),
}

#[derive(Args)]
struct TestPath {
    #[arg(short, long, default_value = ".")]
    /// Path to the ART YAML files
    path: PathBuf,
}

fn parse_vars(s: &str) -> Result<(String, String), String> {
    if !s.contains('=') {
        return Err(
            "Please specify variables using the VARIABLE=VALUE format (no spaces around '=')"
                .to_string(),
        );
    }
    let split: Vec<&str> = s.split('=').collect();

    Ok((split[0].to_string(), split[1].to_string()))
}

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .format_timestamp(None)
        .format_target(false)
        .filter_level(cli.verbose.log_level_filter())
        .init();

    match &cli.command {
        Commands::Run(args) => {
            let vars: HashMap<String, String> = args.vars.clone().into_iter().collect();
            let test_number = (args.test_number - 1) as usize;

            let arr = Arr::new(args.technique.clone(), vars, test_number, args.path.clone());

            if arr.run().is_ok() {
                println!("Test was successful!")
            }

            if args.cleanup {
                if arr.cleanup().is_ok() {
                    println!("Cleanup successeful!")
                }
            }
        }
        Commands::Utils(utils) => match utils {
            Utils::ParseAll(p) => arr::parse_all(&p.path),
            Utils::ListExecutors(p) => match arr::get_all_executors(&p.path) {
                Ok(_) => (),
                Err(e) => eprintln!("{}", e),
            },
            // Utils::RunAll(p) => arr::run_all(&p.path),
        },
        Commands::Cleanup(args) => {
            let vars: HashMap<String, String> = args.vars.clone().into_iter().collect();
            let test_number = (args.test_number - 1) as usize;

            let arr = Arr::new(args.technique.clone(), vars, test_number, args.path.clone());

            if arr.cleanup().is_ok() {
                println!("Success!")
            }
        }
    }
}

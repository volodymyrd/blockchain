use clap::builder::PossibleValuesParser;
use clap::{crate_description, crate_name, Arg, ArgMatches, Command};
use std::error;

fn main() -> Result<(), Box<dyn error::Error>> {
    let matches = app().try_get_matches().unwrap_or_else(|e| e.exit());
    let subcommand = matches.subcommand().unwrap();
    match subcommand {
        ("new", matches) => {
            let word_count = try_get_word_count(matches)?.unwrap();
            println!("word_count {}", word_count)
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn app<'a>() -> Command<'a> {
    Command::new(crate_name!())
        .about(crate_description!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("new")
                .about("Generate new keypair file from a random seed phrase and optional BIP39 passphrase")
                .disable_version_flag(true)
                .arg(
                    Arg::new("outfile")
                        .short('o')
                        .long("outfile")
                        .value_name("FILEPATH")
                        .takes_value(true)
                        .help("Path to generated file"),
                )
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .help("Overwrite the output file if it exists"),
                )
                .arg(
                    Arg::new("silent")
                        .short('s')
                        .long("silent")
                        .help("Do not display seed phrase. Useful when piping output to other programs that prompt for user input, like gpg"),
                )
                .key_generation_common_args()
        )
}

/// clap-v3-utils/keygen/mod.rs
pub trait KeyGenerationCommonArgs {
    fn key_generation_common_args(self) -> Self;
}

impl KeyGenerationCommonArgs for Command<'_> {
    fn key_generation_common_args(self) -> Self {
        self.arg(word_count_arg())
            // .arg(language_arg())
            // .arg(no_passphrase_arg())
    }
}

/// clap-v3-utils/lib.rs
pub struct ArgConstant<'a> {
    pub long: &'a str,
    pub name: &'a str,
    pub help: &'a str,
}
/// clap-v3-utils/keygen/mnemonic.rs
pub const WORD_COUNT_ARG: ArgConstant<'static> = ArgConstant {
    long: "word-count",
    name: "word_count",
    help: "Specify the number of words that will be present in the generated seed phrase",
};

// The constant `POSSIBLE_WORD_COUNTS` and function `try_get_word_count` must always be updated in
// sync
const POSSIBLE_WORD_COUNTS: &[&str] = &["12", "15", "18", "21", "24"];
pub fn word_count_arg<'a>() -> Arg<'a> {
    Arg::new(WORD_COUNT_ARG.name)
        .long(WORD_COUNT_ARG.long)
        .value_parser(PossibleValuesParser::new(POSSIBLE_WORD_COUNTS))
        .default_value("12")
        .value_name("NUMBER")
        .takes_value(true)
        .help(WORD_COUNT_ARG.help)
}
pub fn try_get_word_count(matches: &ArgMatches) -> Result<Option<usize>, Box<dyn error::Error>> {
    Ok(matches
        .try_get_one::<String>(WORD_COUNT_ARG.name)?
        .map(|count| match count.as_str() {
            "12" => 12,
            "15" => 15,
            "18" => 18,
            "21" => 21,
            "24" => 24,
            _ => unreachable!(),
        }))
}

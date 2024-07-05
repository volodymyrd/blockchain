use {
    bip39::{Mnemonic, MnemonicType, Seed},
    clap::{crate_description, crate_name, Arg, Command},
    my_solana_clap_v3_utils::keygen::{
        derivation_path::{acquire_derivation_path, derivation_path_arg},
        mnemonic::{
            acquire_passphrase_and_message, language_arg, no_passphrase_arg, try_get_language,
            try_get_word_count, word_count_arg,
        },
    },
    my_solana_sdk::signature::{keypair_from_seed, keypair_from_seed_and_derivation_path, Signer},
    std::error,
};

fn main() -> Result<(), Box<dyn error::Error>> {
    let matches = app().try_get_matches().unwrap_or_else(|e| e.exit());
    let subcommand = matches.subcommand().unwrap();
    match subcommand {
        ("new", matches) => {
            let word_count = try_get_word_count(matches)?.unwrap();
            let mnemonic_type = MnemonicType::for_word_count(word_count)?;
            let language = try_get_language(matches)?.unwrap();

            let silent = matches.try_contains_id("silent")?;
            if !silent {
                println!("Generating a new keypair");
            }

            let derivation_path = acquire_derivation_path(matches)?;

            let mnemonic = Mnemonic::new(mnemonic_type, language);
            let (passphrase, passphrase_message) = acquire_passphrase_and_message(matches)
                .map_err(|err| format!("Unable to acquire passphrase: {err}"))?;

            let seed = Seed::new(&mnemonic, &passphrase);
            let keypair = match derivation_path {
                Some(_) => keypair_from_seed_and_derivation_path(seed.as_bytes(), derivation_path),
                None => keypair_from_seed(seed.as_bytes()),
            }?;

            if !silent {
                let phrase: &str = mnemonic.phrase();
                let divider = String::from_utf8(vec![b'='; phrase.len()]).unwrap();
                println!(
                    "{}\n{:?}\npubkey: {}\n{}\nSave this seed phrase{} to recover your new keypair:\n{}\n{}",
                    &divider, keypair.secret(), keypair.pubkey(), &divider, passphrase_message, phrase, &divider
                );
            }
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
                .arg(
                    derivation_path_arg()
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
            .arg(language_arg())
            .arg(no_passphrase_arg())
    }
}

//! Loading signers and keypairs from the command line.
//!
//! This module contains utilities for loading [Signer]s and [Keypair]s from
//! standard signing sources, from the command line, as in the Solana CLI.
//!
//! The key function here is [`signer_from_path`], which loads a `Signer` from
//! one of several possible sources by interpreting a "path" command line
//! argument. Its documentation includes a description of all possible signing
//! sources supported by the Solana CLI. Many other functions here are
//! variations on, or delegate to, `signer_from_path`.

use bip39::{Language, Mnemonic, Seed};
use {
    my_solana_sdk::derivation_path::DerivationPath,
    my_solana_sdk::signer::{
        keypair::generate_seed_from_seed_phrase_and_passphrase, keypair::Keypair, EncodableKey,
        EncodableKeypair, SeedDerivable,
    },
    rpassword::prompt_password,
    std::{
        error,
        io::{stdin, stdout, Write},
        process::exit,
    },
};

/// Prompts user for a passphrase and then asks for confirmirmation to check for mistakes
pub fn prompt_passphrase(prompt: &str) -> Result<String, Box<dyn error::Error>> {
    let passphrase = prompt_password(prompt)?;
    if !passphrase.is_empty() {
        let confirmed = rpassword::prompt_password("Enter same passphrase again: ")?;
        if confirmed != passphrase {
            return Err("Passphrases did not match".into());
        }
    }
    Ok(passphrase)
}

/// Reads user input from stdin to retrieve a seed phrase and passphrase for keypair derivation.
///
/// Optionally skips validation of seed phrase. Optionally confirms recovered
/// public key.
pub fn keypair_from_seed_phrase(
    keypair_name: &str,
    skip_validation: bool,
    confirm_pubkey: bool,
    derivation_path: Option<DerivationPath>,
    legacy: bool,
) -> Result<Keypair, Box<dyn error::Error>> {
    let keypair: Keypair =
        encodable_key_from_seed_phrase(keypair_name, skip_validation, derivation_path, legacy)?;
    if confirm_pubkey {
        confirm_encodable_keypair_pubkey(&keypair, "pubkey");
    }
    Ok(keypair)
}

fn encodable_key_from_seed_phrase<K: EncodableKey + SeedDerivable>(
    key_name: &str,
    skip_validation: bool,
    derivation_path: Option<DerivationPath>,
    legacy: bool,
) -> Result<K, Box<dyn error::Error>> {
    let seed_phrase = prompt_password(format!("[{key_name}] seed phrase: "))?;
    let seed_phrase = seed_phrase.trim();
    let passphrase_prompt = format!(
        "[{key_name}] If this seed phrase has an associated passphrase, enter it now. Otherwise, press ENTER to continue: ",
    );

    let key = if skip_validation {
        let passphrase = prompt_passphrase(&passphrase_prompt)?;
        if legacy {
            K::from_seed_phrase_and_passphrase(seed_phrase, &passphrase)?
        } else {
            let seed = generate_seed_from_seed_phrase_and_passphrase(seed_phrase, &passphrase);
            K::from_seed_and_derivation_path(&seed, derivation_path)?
        }
    } else {
        let sanitized = sanitize_seed_phrase(seed_phrase);
        let parse_language_fn = || {
            for language in &[
                Language::English,
                Language::ChineseSimplified,
                Language::ChineseTraditional,
                Language::Japanese,
                Language::Spanish,
                Language::Korean,
                Language::French,
                Language::Italian,
            ] {
                if let Ok(mnemonic) = Mnemonic::from_phrase(&sanitized, *language) {
                    return Ok(mnemonic);
                }
            }
            Err("Can't get mnemonic from seed phrases")
        };
        let mnemonic = parse_language_fn()?;
        let passphrase = prompt_passphrase(&passphrase_prompt)?;
        let seed = Seed::new(&mnemonic, &passphrase);
        if legacy {
            K::from_seed(seed.as_bytes())?
        } else {
            K::from_seed_and_derivation_path(seed.as_bytes(), derivation_path)?
        }
    };
    Ok(key)
}

fn sanitize_seed_phrase(seed_phrase: &str) -> String {
    seed_phrase
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

fn confirm_encodable_keypair_pubkey<K: EncodableKeypair>(keypair: &K, pubkey_label: &str) {
    let pubkey = keypair.encodable_pubkey().to_string();
    println!("Recovered {pubkey_label} `{pubkey:?}`. Continue? (y/n): ");
    let _ignored = stdout().flush();
    let mut input = String::new();
    stdin().read_line(&mut input).expect("Unexpected input");
    if input.to_lowercase().trim() != "y" {
        println!("Exiting");
        exit(1);
    }
}

use clap::{crate_description, crate_name, Command};

fn main() {
    app().try_get_matches().unwrap_or_else(|e| e.exit());
}

fn app<'a>() -> Command<'a> {
    Command::new(crate_name!())
        .about(crate_description!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("new").about(
            "Generate new keypair file from a random seed phrase and optional BIP39 passphrase",
        ))
}

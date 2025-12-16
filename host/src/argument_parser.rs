use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct HostArguments {
    /// Path to the WASM component
    #[arg(index = 1)]
    pub component_path: String,

    /// Path to the GPIO policy TOML file
    #[arg(long = "policy-file")]
    pub policy_file: String,
}

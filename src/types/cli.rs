use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "encdec", about = "decrypt and serve encrypted files")]
#[structopt(version = option_env!("VERSION").unwrap_or(env!("CARGO_PKG_VERSION")))]
pub struct Opt {
    /// Path to config yml file
    #[structopt(short, long)]
    pub config: Option<String>,
    /// Input Directory
    #[structopt(short, long)]
    pub input: Option<String>,
    /// Output Directory
    #[structopt(short, long)]
    pub output: Option<String>,
    #[structopt(subcommand)]
    pub cmd: Option<Command>,
}

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Generate Secrets
    Gen,
    /// Decrypt Directory
    Dec,
    /// Encrypt Directory
    Enc,
}

impl Opt {
    pub fn get_args() -> Self {
        Self::from_args()
    }
}

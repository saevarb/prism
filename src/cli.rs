use clap::Parser;

const TURBO_REGEX: &str = r"^(?P<prefix>\S*?):(?P<rest> .*)";
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
  /// Regex for the prefix
  #[arg(short, long, default_value_t = TURBO_REGEX.to_string())]
  pub prefix: String,

  /// Command to run
  pub command: Vec<String>,
}

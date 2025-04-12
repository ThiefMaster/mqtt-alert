use std::path::PathBuf;

use clap::Parser;
use clap::builder::{
    Styles,
    styling::{AnsiColor, Effects},
};
use clap_verbosity_flag::{Verbosity, WarnLevel};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles = STYLES)]
pub struct CliArgs {
    /// Config file to use
    #[arg(long, short, default_value = "config.toml")]
    pub config: PathBuf,

    /// Enable verbose output
    #[command(flatten)]
    pub verbosity: Verbosity<WarnLevel>,
}

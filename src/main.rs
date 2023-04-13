use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Ok, Result};
use clap::Parser as _;

#[derive(Debug, clap::Parser)]
pub struct Args {
    // TODO: Implemet some error handligns:
    // It must be a file, not a directory.
    // And its length should be greater than 0.
    file: Vec<PathBuf>,
    #[clap(short, long)]
    width: Option<usize>,
    #[clap(short, long)]
    height: Option<usize>,
    /// The color palette file.
    // TODO: Not implemented yet. YAML format?
    #[clap(short, long)]
    colors: Option<PathBuf>,
    /// The character to use for brush.
    /// Default is `█`.
    #[clap(short, long)]
    brush: Option<String>,
}

fn draw<T: Write + ?Sized>(out: &mut T) -> Result<()> {
    out.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut stdout = std::io::stdout();

    draw(&mut stdout)?;

    Ok(())
}

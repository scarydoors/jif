use std::fs::File;
use anyhow::Result;

mod ppm_writer;
mod parser;

use parser::Decoder;

fn main() -> Result<()> {
    env_logger::init();
    let mut file = File::open("./homeless-nah-id-win.gif")?;

    Ok(())
}

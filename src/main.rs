use anyhow::Result;
use std::fs::File;

mod parser;
mod ppm_writer;

use parser::Decoder;

fn main() -> Result<()> {
    env_logger::init();
    let mut file = File::open("./homeless-nah-id-win.gif")?;

    Ok(())
}

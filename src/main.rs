use std::fs::File;
use anyhow::Result;

mod ppm_writer;
mod parser;
mod gfx;

use parser::Decoder;

fn spit_out_gif() -> Result<()> {
    let mut file = File::open("./homeless-nah-id-win.gif")?;

    let mut parser = Decoder::new(&mut file);
    parser.parse()?;

    for (i, frame) in parser.frames().iter().enumerate() {
        let color_table = frame.palette().expect("expected there to be a color palette of some sort");

        let width = frame.width;
        let height = frame.height;
        let indexes = frame.indicies();
        //indexes.chunks(10).for_each(|chunk|  println!("{:?}", chunk));
        ppm_writer::write_ppm(&format!("yeah/frame_{}.ppm", i), width, height, indexes, color_table)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();
    pollster::block_on(gfx::run());
    Ok(())
}

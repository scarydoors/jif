use std::fs::File;
use anyhow::Result;

mod ppm_writer;
mod reader_new;
use reader_new::Parser;


fn main() -> Result<()> {
    env_logger::init();
    let mut file = File::open("./homeless-nah-id-win.gif")?;

    let mut parser = Parser::new(&mut file);
    parser.parse()?;

    for (i, block) in parser.graphic_blocks.iter().enumerate() {
        let color_table = if let Some(color_table) = block.render_block.local_color_table.as_ref() {
            color_table
        } else {
            parser.global_color_table.as_ref().unwrap()
        };

        let width = block.render_block.width;
        let height = block.render_block.height;
        let indexes = block.render_block.image_indexes.as_ref().unwrap();
        //indexes.chunks(10).for_each(|chunk|  println!("{:?}", chunk));
        ppm_writer::write_ppm(&format!("yeah/frame_{}.ppm", i), width, height, indexes.as_ref(), color_table.as_ref())?;
    }
    Ok(())
}

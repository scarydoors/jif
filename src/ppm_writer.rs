use std::io::{prelude::*, BufWriter};
use std::fs::File;
use anyhow::Result;

const MAGIC_NUMBER: &[u8] = b"P3";

pub fn write_ppm(filename: &str, width: u16, height: u16, indexes: &[u8], color_table: &[u8]) -> Result<()> {
    let file = File::create(filename)?;
    //println!("writing {filename}");

    let mut writer = BufWriter::new(&file);

    writer.write(MAGIC_NUMBER)?;
    writer.write(b"\n")?;
    writer.write(format!("{} {}", width, height).as_bytes())?;
    writer.write(b" 255")?;
    writer.write(b"\n")?;

    for index_row in indexes.chunks(width as usize) {
        index_row.iter().enumerate().try_for_each(|(i, idx)| -> Result<()> {
            let color_idx = (*idx as usize) * 3;
            let red = color_table.get(color_idx).unwrap();
            let green = color_table.get(color_idx+1).unwrap();
            let blue = color_table.get(color_idx+2).unwrap();

            writer.write(format!("{: >3} {: >3} {: >3}", red, green, blue).as_bytes())?;
            if i != (width - 1).into() {
                writer.write(b" ")?;
            }
            Ok(())
        })?;
        writer.write(b"\n")?;
    }

    Ok(())
}

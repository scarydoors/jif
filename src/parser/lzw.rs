use super::bit_reader::BitReader;

pub fn lzw_decode(buf: &[u8], minimum_code_size: u32) -> Vec<u8> {
    let mut code_table = init_code_table(minimum_code_size);

    let clear_code: u16 = 1 << minimum_code_size;
    let end_of_information_code = clear_code + 1;
    println!("clear_code={clear_code} end_of_information_code={end_of_information_code}");

    let mut reader = BitReader::new(buf);
    let mut code_size = minimum_code_size + 1;

    let mut indicies: Vec<u8> = Vec::new();

    reader.next(code_size).unwrap();
    let mut last_code = reader.next(code_size).unwrap() as usize;

    let last_code_indicies = code_table.get(last_code as usize).unwrap().clone();

    //println!("read {last_code}, length: {}, code_sz: {code_size}", code_table.len());
    // output the first code
    indicies.extend_from_slice(&last_code_indicies);

    // does code exist in the string table
    while let Some(code) = reader.next(code_size) {
        if code_table.len() == (1 << code_size) - 1 && code_size < 12 {
            code_size += 1;
        }

        if code == clear_code.into() {
            println!("cleared");
            code_size = minimum_code_size + 1;
            code_table = init_code_table(minimum_code_size);
            last_code = reader.next(code_size).unwrap() as usize;
            let last_code_indicies = code_table.get(last_code as usize).unwrap().clone();
            indicies.extend_from_slice(&last_code_indicies);
            continue;
        }

        if code == end_of_information_code.into() {
            break;
        }

        match code_table.get(code as usize) {
            Some(code_indicies) => {
                // println!("Y found: read {code} {code:0b}, length: {}, code_sz: {code_size}", code_table.len());
                // output {CODE} to index stream
                indicies.extend_from_slice(code_indicies);


                // get {CODE-1}
                let mut new_code_table_entry = code_table.get(last_code).unwrap().clone();
                //
                // let K be the first index in {CODE}
                let first_index_of_current_code = code_indicies.first().unwrap();
                // get {CODE-1}+K
                new_code_table_entry.push(*first_index_of_current_code);

                //println!("adding indicies in found path: {new_code_table_entry:?}");
                // add {CODE-1}+K to the code table
                code_table.push(new_code_table_entry);

                // CODE-1 = CODE
                last_code = code as usize;
            },
            None => {
                //println!("N found: read {code} {code:0b}, length: {}, code_sz: {code_size}", code_table.len());
                // {CODE-1}
                let mut new_code_table_entry = code_table.get(last_code).unwrap().clone();
                // let K be the first index of {CODE-1}
                let first_index_of_last_code = new_code_table_entry.first().unwrap();
                // get {CODE-1}+K
                new_code_table_entry.push(*first_index_of_last_code);

                // output {CODE-1}+K to index stream
                indicies.extend_from_slice(&new_code_table_entry);

                //println!("adding indicies in NOT found path: {new_code_table_entry:?}");
                // add {CODE-1}+K to code table
                code_table.push(new_code_table_entry);

                // CODE-1 = CODE
                last_code = code as usize;
            }
        }

    }

    println!("min code size: {}", minimum_code_size);
    //println!("{:?}", code_table);
    indicies
}

fn init_code_table(minimum_code_size: u32) -> Vec<Vec<u8>> {
    let min_table_length: u16 = (1 << minimum_code_size) + 1;
    (0..=min_table_length).map(|i| vec![i as u8]).collect()
}

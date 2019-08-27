use std::env;

use byteorder::{BigEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufReader, Read};
use std::str;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let file = File::open(filename)?;

    println!("Analyzing file '{}'", filename);
    read(BufReader::new(file))?;
    Ok(())
}

fn read(mut reader: BufReader<File>) -> Result<(), Box<dyn std::error::Error>> {
    let mut iff_header = [0u8; 4];
    reader.read_exact(&mut iff_header)?;
    let size = reader.read_u32::<BigEndian>()?;
    let mut form_type = [0u8; 4];
    reader.read_exact(&mut form_type)?;

    println!(
        "Iff: {:?}\nForm: {:?}\nSize: {} bytes\n",
        str::from_utf8(&iff_header)?,
        str::from_utf8(&form_type)?,
        size
    );
    Ok(())
}

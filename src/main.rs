use byteorder::{BigEndian, ReadBytesExt};
use std::convert::TryInto;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};
use std::io::{Seek, SeekFrom};
use std::str;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let file = File::open(filename)?;

    let mut reader = BufReader::new(file);
    let beam_data = BeamData::from_file(&mut reader)?;
    println!("Parsed beam file: {:?}", beam_data);
    Ok(())
}

#[derive(Debug)]
struct BeamData {
    size: u32,
}

#[derive(Debug)]
struct Chunk {
    name: [u8; 4],
    size: u32,
    data: Vec<u8>,
}

fn read_header(reader: &mut BufReader<File>) -> Result<u32, Box<dyn std::error::Error>> {
    let mut iff_header = [0u8; 4];
    reader.read_exact(&mut iff_header)?;
    assert_eq!(b"FOR1", &iff_header);
    let size = reader.read_u32::<BigEndian>()?;
    let mut form_type = [0u8; 4];
    reader.read_exact(&mut form_type)?;
    assert_eq!(b"BEAM", &form_type);

    Ok(size)
}

fn read_chunk(reader: &mut BufReader<File>) -> Result<Chunk, Box<dyn std::error::Error>> {
    let mut name = [0u8; 4];
    reader.read_exact(&mut name)?;
    let size = reader.read_u32::<BigEndian>()?;

    let mut data = vec![0u8; size.try_into()?];
    reader.read_exact(&mut data[..])?;

    // Beam files are padded to always occur on 4-byte boundaries. So we need to see if there is any padding
    // we need to skip here.
    let padding = (4 - (size % 4)) % 4;
    reader.seek(SeekFrom::Current(padding.try_into()?))?;

    Ok(Chunk { name, size, data })
}

impl BeamData {
    pub fn from_file(reader: &mut BufReader<File>) -> Result<BeamData, Box<dyn std::error::Error>> {
        let size = read_header(reader)?;

        let chunk = read_chunk(reader)?;
        println!(
            "Found '{}' chunk | {} bytes",
            str::from_utf8(&chunk.name)?,
            chunk.size
        );

        let chunk = read_chunk(reader)?;
        println!(
            "Found '{}' chunk | {} bytes",
            str::from_utf8(&chunk.name)?,
            chunk.size
        );

        let chunk = read_chunk(reader)?;
        println!(
            "Found '{}' chunk | {} bytes",
            str::from_utf8(&chunk.name)?,
            chunk.size
        );

        let chunk = read_chunk(reader)?;
        println!(
            "Found '{}' chunk | {} bytes",
            str::from_utf8(&chunk.name)?,
            chunk.size
        );

        Ok(BeamData { size })
    }
}

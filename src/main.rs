use byteorder::{BigEndian, ReadBytesExt};
use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::str;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let file = File::open(filename)?;

    let mut reader = BufReader::new(file);
    let beam_data = BeamData::from_file(&mut reader)?;
    println!("Parsed beam file: {:#?}", beam_data);
    Ok(())
}

#[derive(Debug, Default)]
struct BeamData {
    size: u32,
    atoms: Vec<String>,
    exports: Vec<Export>,
}

#[derive(Debug, Default)]
struct Export {
    name: String,
    arity: u32,
    label: u32
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

    // Exclude the "BEAM" constant and return the number of bytes _left_ to read
    Ok(size - 4)
}

fn read_chunk(reader: &mut BufReader<File>) -> Result<(Chunk, u32), Box<dyn std::error::Error>> {
    let mut name = [0u8; 4];
    reader.read_exact(&mut name)?;
    let size = reader.read_u32::<BigEndian>()?;

    let mut data = vec![0u8; size as usize];
    reader.read_exact(&mut data[..])?;

    // Beam files are padded to always occur on 4-byte boundaries. So we need to see if there is any padding
    // we need to skip here.
    let padding = (4 - (size % 4)) % 4;
    reader.seek(SeekFrom::Current(padding as i64))?;

    // Total bytes read is name field (4 bytes) + size field (4 bytes) + size of chunk + padding
    Ok((Chunk { name, size, data }, 4 + 4 + size + padding))
}

impl BeamData {
    fn new(size: u32) -> BeamData {
        BeamData {
            size,
            atoms: Vec::new(),
            exports: Vec::new(),
        }
    }

    pub fn from_file(reader: &mut BufReader<File>) -> Result<BeamData, Box<dyn std::error::Error>> {
        let size_to_read = read_header(reader)?;

        let mut chunks = Vec::new();
        let mut total_read = 0;
        while total_read < size_to_read {
            let (chunk, bytes_read) = read_chunk(reader)?;
            total_read += bytes_read;
            println!(
                "Found '{}' chunk | {} bytes",
                str::from_utf8(&chunk.name)?,
                chunk.size
            );
            chunks.push(chunk);
        }

        let mut data = BeamData::new(size_to_read);
        data.parse_chunks(chunks)?;
        Ok(data)
    }

    fn parse_chunks(&mut self, chunks: Vec<Chunk>) -> Result<(), Box<dyn std::error::Error>> {
        for chunk in chunks {
            match &chunk.name {
                b"AtU8" => self.parse_atoms(chunk)?,
                b"Atom" => self.parse_atoms(chunk)?,
                b"ExpT" => self.parse_exports(chunk)?,
                _ => {
                    // Ignore unrecognized chunk
                }
            }
        }

        Ok(())
    }

    // Parses an atom chunk from the raw chunk data. The format of the raw atom chunk data is as follows:
    //      - Number of atoms (32-bits / Big Endian)
    //      - For each atom:
    //          - length in bytes (8-bits)
    //          - atom name (# of bytes specified in length). A string made up of successive utf8 characters
    fn parse_atoms(&mut self, chunk: Chunk) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = &chunk.data[..];
        let natoms = reader.read_u32::<BigEndian>()?;
        for _ in 0..natoms {
            let len = reader.read_u8()?;
            let mut atom = vec![0u8; len as usize];
            reader.read_exact(&mut atom)?;
            self.atoms.push(String::from_utf8(atom)?);
        }
        Ok(())
    }

    // Parses the export table from the raw chunk data. The format of the raw export table is as follows:
    //      - Number of exports (32-bits / Big Endian)
    //      - For each export:
    //          - function name (32-bits / Big Endian). Index into the atom table (note: index is 1-based, not 0-based)
    //          - arity: (32-bits / Big Endian)
    //          - label: (32-bits / Big Endian)
    fn parse_exports(&mut self, chunk: Chunk) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = &chunk.data[..];
        let nexports = reader.read_u32::<BigEndian>()?;
        for _ in 0..nexports {
            let name_index = reader.read_u32::<BigEndian>()?;
            let arity = reader.read_u32::<BigEndian>()?;
            let label = reader.read_u32::<BigEndian>()?;
            println!("name_index: {}", name_index);
            self.exports.push(Export {
                // Indexes into the atom table are 1-based, not 0-based
                name: self.atoms[(name_index - 1) as usize].clone(),
                arity,
                label
            })
        }

        Ok(())
    }
}

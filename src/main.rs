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
    imports: Vec<Import>,
    code: Code,
    string_literals: Vec<u8>,
}

#[derive(Debug, Default)]
struct Export {
    function_name: String,
    arity: u32,
    label: u32,
}

#[derive(Debug, Default)]
struct Import {
    module_name: String,
    function_name: String,
    arity: u32,
}

#[derive(Debug, Default)]
struct Code {
    instruction_set: u32,
    max_opcode: u32,
    label_count: u32,
    function_count: u32,
    opcodes: Vec<u8>
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
    reader.seek(SeekFrom::Current(i64::from(padding)))?;

    // Total bytes read is name field (4 bytes) + size field (4 bytes) + size of chunk + padding
    Ok((Chunk { name, size, data }, 4 + 4 + size + padding))
}

impl BeamData {
    fn new(size: u32) -> BeamData {
        BeamData {
            size,
            atoms: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            code: Default::default(),
            string_literals: Vec::new(),
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
                b"ImpT" => self.parse_imports(chunk)?,
                b"Code" => self.parse_code(chunk)?,
                b"StrT" => self.parse_string_literals(chunk)?,
                _ => {
                    // Ignore unrecognized chunk
                }
            }
        }

        Ok(())
    }

    /// Parses an atom chunk from the raw chunk data. The format of the raw atom chunk data is as follows:
    ///      - Number of atoms (32-bits / Big Endian)
    ///      - For each atom:
    ///          - length in bytes (8-bits)
    ///          - atom name (# of bytes specified in length). A string made up of successive utf8 characters
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

    /// Parses the export table from the raw chunk data. The format of the raw export table is as follows:
    ///      - Number of exports (32-bits / Big Endian)
    ///      - For each export:
    ///          - function name (32-bits / Big Endian). Index into the atom table (note: index is 1-based, not 0-based)
    ///          - arity: (32-bits / Big Endian)
    ///          - label: (32-bits / Big Endian)
    fn parse_exports(&mut self, chunk: Chunk) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = &chunk.data[..];
        let export_count = reader.read_u32::<BigEndian>()?;
        for _ in 0..export_count {
            let name_index = reader.read_u32::<BigEndian>()?;
            let arity = reader.read_u32::<BigEndian>()?;
            let label = reader.read_u32::<BigEndian>()?;
            self.exports.push(Export {
                // Indexes into the atom table are 1-based, not 0-based
                function_name: self.atoms[(name_index - 1) as usize].clone(),
                arity,
                label
            })
        }

        Ok(())
    }

    /// Parses the import table from the raw chunk data. The format of the raw import table is as follows:
    ///      - Number of imports (32-bits / Big Endian)
    ///      - For each export:
    ///          - module name (32-bits / Big Endian). Index into the atom table (note: index is 1-based, not 0-based)
    ///          - function name (32-bits / Big Endian). Index into the atom table (note: index is 1-based, not 0-based)
    ///          - arity: (32-bits / Big Endian)
    fn parse_imports(&mut self, chunk: Chunk) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = &chunk.data[..];
        let import_count = reader.read_u32::<BigEndian>()?;
        for _ in 0..import_count {
            let module_name_index = reader.read_u32::<BigEndian>()?;
            let function_name_index = reader.read_u32::<BigEndian>()?;
            let arity = reader.read_u32::<BigEndian>()?;
            self.imports.push(Import {
                // Indexes into the atom table are 1-based, not 0-based
                module_name: self.atoms[(module_name_index - 1) as usize].clone(),
                function_name: self.atoms[(function_name_index - 1) as usize].clone(),
                arity
            })
        }

        Ok(())
    }

    /// Parses the code chunk from raw chunk data. The format of the code chunk is as follows:
    ///      - Sub-size (32-bits / Big Endian). Tells us how big the next sub-section is. It is done this way so that
    ///             more fields can be added in the future to this sub-section without breaking previous parsers
    ///      - Sub section:
    ///         - Instruction set (32-bits / Big Endian)
    ///         - Maximum opcode (32-bits / Big Endian)
    ///         - Label count (32-bits / Big Endian)
    ///         - Function count (32-bits / Big Endian)
    ///      - The rest of the chunk (minus padding) contains the raw opcodes for the module
    fn parse_code(&mut self, chunk: Chunk) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = &chunk.data[..];
        let sub_size = reader.read_u32::<BigEndian>()?;

        // We need to do this rather than parsing the various values directly as BEAM files
        // use sub_size so that extra fields can be added in the future (where the size of the sub
        // section will end up changing) without breaking existing parsers/runtimes.
        let mut sub_reader = vec![0u8; sub_size as usize];
        reader.read_exact(&mut sub_reader)?;
        let mut sub_data = &sub_reader[..];
        let instruction_set = sub_data.read_u32::<BigEndian>()?;
        let max_opcode = sub_data.read_u32::<BigEndian>()?;
        let label_count = sub_data.read_u32::<BigEndian>()?;
        let function_count = sub_data.read_u32::<BigEndian>()?;

        self.code = Code {
            instruction_set,
            max_opcode,
            label_count,
            function_count,
            opcodes: Vec::new()
        };
        reader.read_to_end(&mut self.code.opcodes)?;

        Ok(())
    }

    /// Parses string literals out of the raw chunk. The string literals are just a flat array of
    /// utf8 characters the size of the chunk.
    fn parse_string_literals(&mut self, chunk: Chunk) -> Result<(), Box<dyn std::error::Error>> {
        if chunk.size > 0 {
            let mut reader = &chunk.data[..];
            self.string_literals = vec![0u8; chunk.size as usize];
            reader.read_exact(&mut self.string_literals)?;
        }

        Ok(())
    }
}

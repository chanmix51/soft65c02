use std::cmp::min;
use crate::commands::MemorySegment;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use anyhow::anyhow;
use crate::AppResult;

#[derive(Debug)]
pub struct AtariBinary {
    bytes: Vec<u8>,
    sections: Vec<Section>,
    run_address: usize,
}

#[derive(Debug, PartialEq)]
pub enum Section {
    Init { init_address: usize },
    Run { run_address: usize },
    Data { start_address: usize, data: Vec<u8> },
}

impl AtariBinary {
    pub fn new(bytes: Vec<u8>) -> Result<Self, String> {
        // Check header is FFFF
        if bytes.len() < 2 || bytes[0] != 0xff || bytes[1] != 0xff {
            let header_hex = bytes.get(0..2)
                .map(|b| b.iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>())
                .unwrap_or_default();
            return Err(format!("Unhandled header type: {}", header_hex));
        }

        let mut binary = AtariBinary {
            bytes,
            sections: Vec::new(),
            run_address: 0,
        };

        let mut i = 0;
        while i < binary.bytes.len() {
            if i + 1 < binary.bytes.len() && binary.bytes[i] == 0xe2 && binary.bytes[i + 1] == 0x02 {
                // INIT, skip 4 bytes [0xe2 0x02 0xe3 0x02]
                i += 4;
                if i + 1 < binary.bytes.len() {
                    let init_address = binary.bytes[i] as usize + 256 * (binary.bytes[i + 1] as usize);
                    binary.sections.push(Section::Init { init_address });
                    i += 2;
                }
            } else if i + 1 < binary.bytes.len() && binary.bytes[i] == 0xe0 && binary.bytes[i + 1] == 0x02 {
                // RUN ADDR, skip 4 bytes [0xe0 0x02 0xe1 0x02]
                i += 4;
                if i + 1 < binary.bytes.len() {
                    let run_address = binary.bytes[i] as usize + 256 * (binary.bytes[i + 1] as usize);
                    binary.run_address = run_address; // Keep for backward compatibility
                    binary.sections.push(Section::Run { run_address });
                    i += 2;
                    // should now be at the end
                    if i != binary.bytes.len() {
                        return Err(format!(
                            "Failed to process file, got run address but there is more data, i: {}, size: {}",
                            i,
                            binary.bytes.len()
                        ));
                    }
                }
            } else {
                // new data block. Header is optional (except first, but this works either way)
                if i + 1 < binary.bytes.len() && binary.bytes[i] == 0xff && binary.bytes[i + 1] == 0xff {
                    i += 2;
                }
                if i + 3 < binary.bytes.len() {
                    let start_address = binary.bytes[i] as usize + 256 * (binary.bytes[i + 1] as usize);
                    let end_address = binary.bytes[i + 2] as usize + 256 * (binary.bytes[i + 3] as usize);
                    let block_len = end_address - start_address + 1;
                    i += 4;
                    if i + block_len <= binary.bytes.len() {
                        let data = binary.bytes[i..i + block_len].to_vec();
                        binary.sections.push(Section::Data {
                            start_address,
                            data,
                        });
                        i += block_len;
                    }
                }
            }
        }

        Ok(binary)
    }

    pub fn dump(&self) {
        println!(
            "AtariBinary, runAddress: 0x{:x}, len: 0x{:x}",
            self.run_address,
            self.bytes.len()
        );
        for section in &self.sections {
            match section {
                Section::Data { start_address, data } => {
                    let up_to_7 = min(7, data.len().saturating_sub(1));
                    let first_8 = data[0..=up_to_7]
                        .iter()
                        .map(|byte| format!("{:02x}", byte))
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!(
                        "  DataSection, start: 0x{:x}, len: 0x{:x}: {}",
                        start_address,
                        data.len(),
                        first_8
                    );
                }
                Section::Init { init_address } => {
                    println!("  InitSection, init: 0x{:x}", init_address);
                }
                Section::Run { run_address } => {
                    println!("  RunSection, run: 0x{:x}", run_address);
                }
            }
        }
    }

    pub fn get_sections(&self) -> &[Section] {
        &self.sections
    }

    pub fn get_run_address(&self) -> usize {
        self.run_address
    }

    /// Convert the binary into a series of memory segments ready to be loaded.
    /// This includes both data sections and control vectors (INIT/RUN).
    pub fn into_memory_segments(&self) -> Vec<MemorySegment> {
        let mut segments = Vec::new();

        // Process all sections including data and control vectors
        for section in &self.sections {
            match section {
                Section::Data { start_address, data } => {
                    segments.push(MemorySegment {
                        address: *start_address,
                        data: data.clone(),
                    });
                }
                Section::Init { init_address } => {
                    // Write INIT vector to $02E2-$02E3
                    segments.push(MemorySegment {
                        address: 0x02E2,
                        data: vec![
                            (*init_address & 0xFF) as u8,
                            ((*init_address >> 8) & 0xFF) as u8
                        ],
                    });
                }
                Section::Run { run_address } => {
                    // Write RUN vector to $02E0-$02E1
                    segments.push(MemorySegment {
                        address: 0x02E0,
                        data: vec![
                            (*run_address & 0xFF) as u8,
                            ((*run_address >> 8) & 0xFF) as u8
                        ],
                    });
                }
            }
        }

        segments
    }

    /// Creates an AtariBinary by reading and parsing a file
    pub fn from_file(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(anyhow!("File not found: {}", path.display()));
        }
        
        let mut f = File::open(path)
            .map_err(|e| anyhow!("Failed to open file {}: {}", path.display(), e))?;
            
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .map_err(|e| anyhow!("Failed to read file {}: {}", path.display(), e))?;
            
        Self::new(buffer).map_err(|e| anyhow!("Failed to parse Atari binary {}: {}", path.display(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a simple atari binary with one data block and run address
    fn create_simple_binary() -> Vec<u8> {
        let binary = vec![
            // Header
            0xff, 0xff,
            // Data block 1: Load at $1000, 5 bytes
            // Start address: $1000 (low byte first)
            0x00, 0x10,
            // End address: $1004 (low byte first)
            0x04, 0x10,
            // Data: 5 bytes of 0x01
            0x01, 0x01, 0x01, 0x01, 0x01,
            // Run address block: $1001
            0xe0, 0x02, 0xe1, 0x02,
            // Run address (low byte first)
            0x01, 0x10,
        ];
        binary
    }

    // Helper to create a complex atari binary with multiple sections
    fn create_complex_binary() -> Vec<u8> {
        let binary = vec![
            // Header
            0xff, 0xff,
            // Data block 1: Load at $1000, 5 bytes
            // Start address: $1000 (low byte first)
            0x00, 0x10,
            // End address: $1004 (low byte first)
            0x04, 0x10,
            // Data: 5 bytes of 0x01
            0x01, 0x01, 0x01, 0x01, 0x01,
            // Init block: $1001
            0xe2, 0x02, 0xe3, 0x02,
            // Init address (low byte first)
            0x01, 0x10,
            // Data block 2: Load at $2000, 5 bytes
            // Start address: $2000 (low byte first)
            0x00, 0x20,
            // End address: $2004 (low byte first)
            0x04, 0x20,
            // Data: 5 bytes of 0x02
            0x02, 0x02, 0x02, 0x02, 0x02,
            // Run address block: $2002
            0xe0, 0x02, 0xe1, 0x02,
            // Run address (low byte first)
            0x02, 0x20,
        ];
        binary
    }

    #[test]
    fn test_simple_binary() {
        let binary = AtariBinary::new(create_simple_binary()).unwrap();
        let sections = binary.get_sections();
        
        // Check we have exactly two sections (data and run)
        assert_eq!(sections.len(), 2);
        
        // Check first section - data at $1000
        match &sections[0] {
            Section::Data { start_address, data } => {
                assert_eq!(*start_address, 0x1000, "Data section should start at $1000");
                assert_eq!(data, &vec![0x01, 0x01, 0x01, 0x01, 0x01], "Data section should contain five 0x01 bytes");
            }
            _ => panic!("First section should be Data, got {:?}", &sections[0])
        }
        
        // Check second section - run address at $1001
        match &sections[1] {
            Section::Run { run_address } => {
                assert_eq!(*run_address, 0x1001, "Run address should be $1001");
            }
            _ => panic!("Second section should be Run, got {:?}", &sections[1])
        }
    }

    #[test]
    fn test_complex_binary() {
        let binary = AtariBinary::new(create_complex_binary()).unwrap();
        let sections = binary.get_sections();
        
        // Check we have exactly four sections (data, init, data, run)
        assert_eq!(sections.len(), 4);
        
        // Check first section - data at $1000
        match &sections[0] {
            Section::Data { start_address, data } => {
                assert_eq!(*start_address, 0x1000, "First data section should start at $1000");
                assert_eq!(data, &vec![0x01, 0x01, 0x01, 0x01, 0x01], "First data section should contain five 0x01 bytes");
            }
            _ => panic!("First section should be Data, got {:?}", &sections[0])
        }
        
        // Check second section - init at $1001
        match &sections[1] {
            Section::Init { init_address } => {
                assert_eq!(*init_address, 0x1001, "Init section should point to $1001");
            }
            _ => panic!("Second section should be Init, got {:?}", &sections[1])
        }
        
        // Check third section - data at $2000
        match &sections[2] {
            Section::Data { start_address, data } => {
                assert_eq!(*start_address, 0x2000, "Second data section should start at $2000");
                assert_eq!(data, &vec![0x02, 0x02, 0x02, 0x02, 0x02], "Second data section should contain five 0x02 bytes");
            }
            _ => panic!("Third section should be Data, got {:?}", &sections[2])
        }
        
        // Check fourth section - run at $2002
        match &sections[3] {
            Section::Run { run_address } => {
                assert_eq!(*run_address, 0x2002, "Run address should be $2002");
            }
            _ => panic!("Fourth section should be Run, got {:?}", &sections[3])
        }
    }

    #[test]
    fn test_invalid_header() {
        let mut bad_binary = create_simple_binary();
        bad_binary[0] = 0x00; // Corrupt the header
        
        let result = AtariBinary::new(bad_binary);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unhandled header type: 00ff"));
    }

    #[test]
    fn test_load_from_hyphenated_path() {
        use tempfile::tempdir;
        
        // Create a temporary directory with a hyphenated path
        let dir = tempdir().unwrap();
        let test_dir = dir.path().join("build");
        std::fs::create_dir(&test_dir).unwrap();
        let test_file = test_dir.join("test-app.bin");
        
        // Create a simple binary file
        let binary_data = vec![
            // Header
            0xff, 0xff,
            // Data block: Load at $2000, 3 bytes
            0x00, 0x20, // Start address $2000
            0x02, 0x20, // End address $2002
            0xA9, 0x42, 0x60, // LDA #$42, RTS
            // Run address: $2000
            0xe0, 0x02, 0xe1, 0x02,
            0x00, 0x20,
        ];
        
        std::fs::write(&test_file, binary_data).unwrap();
        
        // Load and verify the binary
        let binary = AtariBinary::from_file(&test_file).unwrap();
        let sections = binary.get_sections();
        
        assert_eq!(sections.len(), 2);
        
        // Verify data section
        match &sections[0] {
            Section::Data { start_address, data } => {
                assert_eq!(*start_address, 0x2000);
                assert_eq!(data, &vec![0xA9, 0x42, 0x60]);
            }
            _ => panic!("First section should be Data"),
        }
        
        // Verify run address
        match &sections[1] {
            Section::Run { run_address } => {
                assert_eq!(*run_address, 0x2000);
            }
            _ => panic!("Second section should be Run"),
        }
    }
} 
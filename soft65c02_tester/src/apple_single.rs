use std::cmp::min;
use crate::commands::MemorySegment;
use anyhow::anyhow;
use crate::AppResult;
use std::{fs::File, io::Read, path::Path};

/// Entry types defined in the AppleSingle format
#[derive(Debug, PartialEq)]
pub enum ASEntry {
    DataFork { data: Vec<u8> },
    ProDosInfo { 
        access: u16,            // 16 bits
        file_type: u16,         // 16 bits
        auxiliary_type: u32,    // 32 bits
    },
}

#[derive(Debug)]
pub struct AppleSingle {
    bytes: Vec<u8>,
    entries: Vec<ASEntry>,
    load_address: u16,  // Derived from ProDOS auxiliary_type's lower 16 bits
}

impl AppleSingle {
    pub fn new(bytes: Vec<u8>) -> Result<Self, String> {
        // Check magic number (00051600)
        if bytes.len() < 4 || 
           bytes[0] != 0x00 || bytes[1] != 0x05 || 
           bytes[2] != 0x16 || bytes[3] != 0x00 {
            let header_hex = bytes.get(0..4)
                .map(|b| b.iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>())
                .unwrap_or_default();
            return Err(format!("Invalid magic number: {}", header_hex));
        }

        // Check version number (00020000)
        if bytes.len() < 8 || 
           bytes[4] != 0x00 || bytes[5] != 0x02 ||
           bytes[6] != 0x00 || bytes[7] != 0x00 {
            let version_hex = bytes.get(4..8)
                .map(|b| b.iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>())
                .unwrap_or_default();
            return Err(format!("Unsupported version: {}", version_hex));
        }

        // Check filler (16 bytes of zeros)
        if bytes.len() < 24 {
            return Err("File too short for header".to_string());
        }
        for i in 8..24 {
            if bytes[i] != 0 {
                return Err(format!("Invalid filler byte at offset {}", i));
            }
        }

        // Get number of entries (2 bytes, big-endian)
        if bytes.len() < 26 {
            return Err("File too short for entry count".to_string());
        }
        let num_entries = ((bytes[24] as usize) << 8) | (bytes[25] as usize);

        let mut apple_single = AppleSingle {
            bytes,
            entries: Vec::new(),
            load_address: 0,  // Will be set from ProDOS info if present
        };

        // Parse entries
        let mut offset = 26; // Start after header
        for _ in 0..num_entries {
            if offset + 12 > apple_single.bytes.len() {
                return Err("File too short for entry descriptor".to_string());
            }

            let entry_id = ((apple_single.bytes[offset] as u32) << 24) |
                          ((apple_single.bytes[offset + 1] as u32) << 16) |
                          ((apple_single.bytes[offset + 2] as u32) << 8) |
                          (apple_single.bytes[offset + 3] as u32);

            let entry_offset = ((apple_single.bytes[offset + 4] as usize) << 24) |
                             ((apple_single.bytes[offset + 5] as usize) << 16) |
                             ((apple_single.bytes[offset + 6] as usize) << 8) |
                             (apple_single.bytes[offset + 7] as usize);

            let entry_length = ((apple_single.bytes[offset + 8] as usize) << 24) |
                             ((apple_single.bytes[offset + 9] as usize) << 16) |
                             ((apple_single.bytes[offset + 10] as usize) << 8) |
                             (apple_single.bytes[offset + 11] as usize);

            match entry_id {
                1 => { // Data Fork
                    if entry_offset + entry_length > apple_single.bytes.len() {
                        return Err("Data fork entry extends beyond file".to_string());
                    }
                    let data = apple_single.bytes[entry_offset..entry_offset + entry_length].to_vec();
                    apple_single.entries.push(ASEntry::DataFork { data });
                }
                11 => { // ProDOS File Info
                    if entry_length != 8 {
                        return Err(format!("ProDOS info must be 8 bytes, got {}", entry_length));
                    }
                    if entry_offset + entry_length > apple_single.bytes.len() {
                        return Err("ProDOS info entry extends beyond file".to_string());
                    }
                    
                    let access = ((apple_single.bytes[entry_offset] as u16) << 8) |
                                (apple_single.bytes[entry_offset + 1] as u16);
                    let file_type = ((apple_single.bytes[entry_offset + 2] as u16) << 8) |
                                   (apple_single.bytes[entry_offset + 3] as u16);
                    let auxiliary_type = ((apple_single.bytes[entry_offset + 4] as u32) << 24) |
                                       ((apple_single.bytes[entry_offset + 5] as u32) << 16) |
                                       ((apple_single.bytes[entry_offset + 6] as u32) << 8) |
                                       (apple_single.bytes[entry_offset + 7] as u32);
                    
                    // Set load_address from auxiliary_type's lower 16 bits
                    apple_single.load_address = (auxiliary_type & 0xFFFF) as u16;
                    
                    apple_single.entries.push(ASEntry::ProDosInfo {
                        access,
                        file_type,
                        auxiliary_type,
                    });
                }
                _ => return Err(format!("Unsupported entry type: {}", entry_id)),
            }

            offset += 12;
        }

        Ok(apple_single)
    }

    pub fn dump(&self) {
        println!("AppleSingle, loadAddress: 0x{:x}", self.load_address);
        for entry in &self.entries {
            match entry {
                ASEntry::DataFork { data } => {
                    let up_to_7 = min(7, data.len().saturating_sub(1));
                    let first_8 = data[0..=up_to_7]
                        .iter()
                        .map(|byte| format!("{:02x}", byte))
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!(
                        "  DataForkEntry, len: 0x{:x}: {}",
                        data.len(),
                        first_8
                    );
                }
                ASEntry::ProDosInfo { access, file_type, auxiliary_type } => {
                    println!(
                        "  ProDosEntry, access: 0x{:x}, fileType: 0x{:x}, auxiliaryType: 0x{:x}",
                        access, file_type, auxiliary_type
                    );
                }
            }
        }
    }

    pub fn get_entries(&self) -> &[ASEntry] {
        &self.entries
    }

    pub fn into_memory_segments(&self) -> Vec<MemorySegment> {
        let mut segments = Vec::new();

        // For now, we only care about the data fork
        for entry in &self.entries {
            if let ASEntry::DataFork { data } = entry {
                segments.push(MemorySegment {
                    address: self.load_address as usize,  // Convert u16 to usize
                    data: data.clone(),
                });
            }
        }

        segments
    }

    /// Creates an AppleSingle by reading and parsing a file
    pub fn from_file(path: impl AsRef<Path>) -> AppResult<Self> {
        let mut f = File::open(path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        Self::new(buffer).map_err(|e| anyhow!(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_simple_apple_single() -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Magic number (00051600)
        bytes.extend_from_slice(&[0x00, 0x05, 0x16, 0x00]);
        
        // Version (00020000)
        bytes.extend_from_slice(&[0x00, 0x02, 0x00, 0x00]);
        
        // Filler (16 zeros)
        bytes.extend_from_slice(&[0x00; 16]);
        
        // Number of entries (2)
        bytes.extend_from_slice(&[0x00, 0x02]);
        
        // Entry 1: Data Fork
        bytes.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x01,  // Entry ID
            0x00, 0x00, 0x00, 0x32,  // Offset (50)
            0x00, 0x00, 0x00, 0x05,  // Length (5 bytes)
        ]);
        
        // Entry 2: ProDOS Info
        bytes.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x0B,  // Entry ID
            0x00, 0x00, 0x00, 0x37,  // Offset (55)
            0x00, 0x00, 0x00, 0x08,  // Length (8 bytes)
        ]);
        
        // Data Fork content
        bytes.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        
        // ProDOS Info content with custom load address 0x1234
        bytes.extend_from_slice(&[
            0x00, 0xC3,  // Access
            0x00, 0xFF,  // File type
            0x00, 0x00,  // Auxiliary type high bytes
            0x12, 0x34,  // Auxiliary type low bytes (becomes load address)
        ]);
        
        bytes
    }

    #[test]
    fn test_simple_apple_single() {
        let binary = AppleSingle::new(create_simple_apple_single()).unwrap();
        let entries = binary.get_entries();
        
        assert_eq!(entries.len(), 2);
        
        match &entries[0] {
            ASEntry::DataFork { data } => {
                assert_eq!(data, &vec![0x01, 0x02, 0x03, 0x04, 0x05]);
            }
            _ => panic!("First entry should be DataFork"),
        }
        
        match &entries[1] {
            ASEntry::ProDosInfo { access, file_type, auxiliary_type } => {
                assert_eq!(*access, 0xC3);
                assert_eq!(*file_type, 0xFF);
                assert_eq!(*auxiliary_type, 0x00001234);
            }
            _ => panic!("Second entry should be ProDosInfo"),
        }
    }

    #[test]
    fn test_invalid_magic_number() {
        let mut bad_binary = create_simple_apple_single();
        bad_binary[0] = 0xFF;
        
        let result = AppleSingle::new(bad_binary);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid magic number"));
    }

    #[test]
    fn test_into_memory_segments() {
        let binary = AppleSingle::new(create_simple_apple_single()).unwrap();
        let segments = binary.into_memory_segments();
        
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].address, 0x1234, "Load address should match auxiliary type's lower 16 bits");
        assert_eq!(segments[0].data, vec![0x01, 0x02, 0x03, 0x04, 0x05]);
    }

    #[test]
    fn test_load_real_apple_single() {
        let filepath = PathBuf::new().join("tests/apple.bin");
        let binary = AppleSingle::from_file(filepath).unwrap();
        let entries = binary.get_entries();
        
        // We expect at least one entry (the data fork)
        assert!(!entries.is_empty(), "File should contain at least one entry");
        
        // Check each entry type and content
        let mut found_data_fork = false;
        let mut found_prodos_info = false;
        
        // Verify the load_address is correctly set from ProDOS auxiliary_type
        assert_eq!(binary.load_address, 0x0803, "Load address should be 0x0803 (from auxiliary_type's lower 16 bits)");
        
        for entry in entries {
            match entry {
                ASEntry::DataFork { data } => {
                    found_data_fork = true;
                    assert!(!data.is_empty(), "Data fork should not be empty");
                    
                    // Check the first 16 bytes of the data fork
                    let expected_first_bytes = vec![
                        0xa2, 0xff, 0x9a, 0x20, 0x69, 0x13, 0x20, 0x5b,
                        0x11, 0x4c, 0x71, 0x0c, 0x2c, 0x82, 0xc0, 0x20
                    ];
                    assert!(data.len() >= 16, "Data fork should have at least 16 bytes");
                    assert_eq!(&data[..16], &expected_first_bytes, "First 16 bytes of data fork don't match expected values");
                    
                    // println!("Data fork first bytes: {:02x?}", &data[..min(16, data.len())]);
                }
                ASEntry::ProDosInfo { ref access, ref file_type, ref auxiliary_type } => {
                    found_prodos_info = true;
                    
                    // Check ProDOS info values
                    let expected_access = 0xc3_u16;
                    let expected_file_type = 0x06_u16;
                    let expected_aux_type = 0x803_u32;
                    
                    assert_eq!(*access, expected_access, "ProDOS access value mismatch");
                    assert_eq!(*file_type, expected_file_type, "ProDOS file type mismatch");
                    assert_eq!(*auxiliary_type, expected_aux_type, "ProDOS auxiliary type mismatch");
                    
                    // Verify load_address matches lower 16 bits of auxiliary_type
                    assert_eq!(binary.load_address, (*auxiliary_type & 0xFFFF) as u16, 
                             "Load address should match lower 16 bits of auxiliary_type");
                    
                    // println!("ProDOS Info - Access: 0x{:04x}, File Type: 0x{:04x}, Aux Type: 0x{:08x}",
                    //         access, file_type, auxiliary_type);
                }
            }
        }
        
        assert!(found_data_fork, "File should contain a data fork");
        assert!(found_prodos_info, "File should contain ProDOS info");
    }
} 
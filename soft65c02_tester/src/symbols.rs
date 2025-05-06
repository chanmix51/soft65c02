use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    // Map from address to list of symbols at that address
    symbols: HashMap<u16, Vec<String>>,
    // Map from symbol name to address for reverse lookup
    addresses: HashMap<String, u16>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            addresses: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.addresses.len()
    }

    pub fn load_vice_labels<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if let Some(symbol) = self.parse_vice_label_line(&line) {
                let (addr, name) = symbol;
                self.add_symbol(addr, name);
            }
        }
        Ok(())
    }

    fn parse_vice_label_line(&self, line: &str) -> Option<(u16, String)> {
        // Format: al XXXXXX .name (where X is a hex digit)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 || parts[0] != "al" || !parts[2].starts_with('.') {
            return None;
        }

        // Parse the address - for 6-digit hex numbers, we want the last 4 digits
        let addr_str = parts[1];
        if addr_str.len() < 4 {
            return None;
        }
        let addr_str = &addr_str[addr_str.len() - 4..];
        if let Ok(addr) = u16::from_str_radix(addr_str, 16) {
            // Remove the leading dot from the name
            let name = parts[2][1..].to_string();
            Some((addr, name))
        } else {
            None
        }
    }

    pub fn add_symbol(&mut self, addr: u16, name: String) {
        // If symbol already exists, remove it from old address's symbol list
        if let Some(old_addr) = self.addresses.get(&name) {
            if let Some(symbols) = self.symbols.get_mut(old_addr) {
                symbols.retain(|s| s != &name);
                // Remove the Vec if it's empty
                if symbols.is_empty() {
                    self.symbols.remove(old_addr);
                }
            }
        }
        
        // Add to new location
        self.symbols.entry(addr).or_default().push(name.clone());
        self.addresses.insert(name, addr);
    }

    pub fn get_symbols_at(&self, addr: u16) -> Option<&Vec<String>> {
        let result = self.symbols.get(&addr);
        result
    }

    pub fn get_address(&self, symbol: &str) -> Option<u16> {
        let result = self.addresses.get(symbol).copied();
        result
    }

    pub fn dump(&self) {
        for (addr, symbols) in &self.symbols {
            println!("${:04X}: {}", addr, symbols.join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_vice_label() {
        let table = SymbolTable::new();
        
        // Valid lines
        let line = "al 0803 .start";
        assert_eq!(table.parse_vice_label_line(line), Some((0x0803, "start".to_string())));
        
        let line = "al 002000 .main";
        assert_eq!(table.parse_vice_label_line(line), Some((0x2000, "main".to_string())));

        // Invalid lines
        assert_eq!(table.parse_vice_label_line("invalid"), None);
        assert_eq!(table.parse_vice_label_line("al invalid .name"), None);
        assert_eq!(table.parse_vice_label_line("al 0803 name"), None);
        assert_eq!(table.parse_vice_label_line("al 123 .short"), None);  // Too few digits
    }

    #[test]
    fn test_load_vice_labels() -> io::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "al 0803 .start")?;
        writeln!(file, "al 0804 .loop")?;
        writeln!(file, "al 0803 .entry")?; // Multiple symbols for same address
        writeln!(file, "invalid line")?;
        writeln!(file, "al 0805 name")?; // Invalid format
        file.flush()?;

        let mut table = SymbolTable::new();
        table.load_vice_labels(file.path())?;

        assert_eq!(table.get_address("start"), Some(0x0803));
        assert_eq!(table.get_address("loop"), Some(0x0804));
        assert_eq!(table.get_address("entry"), Some(0x0803));

        let symbols_at_803 = table.get_symbols_at(0x0803).unwrap();
        assert!(symbols_at_803.contains(&"start".to_string()));
        assert!(symbols_at_803.contains(&"entry".to_string()));

        Ok(())
    }

    #[test]
    fn test_symbol_reassignment() {
        let mut table = SymbolTable::new();
        
        // Add symbol first time
        table.add_symbol(0x1234, "test".to_string());
        assert_eq!(table.get_address("test"), Some(0x1234));
        assert!(table.get_symbols_at(0x1234).unwrap().contains(&"test".to_string()));
        
        // Reassign symbol to new address
        table.add_symbol(0x5678, "test".to_string());
        
        // Check new location
        assert_eq!(table.get_address("test"), Some(0x5678));
        assert!(table.get_symbols_at(0x5678).unwrap().contains(&"test".to_string()));
        
        // Check old location is cleaned up
        assert!(table.get_symbols_at(0x1234).is_none() || !table.get_symbols_at(0x1234).unwrap().contains(&"test".to_string()));
    }
} 
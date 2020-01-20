use super::*;
use fmt::Debug;
use range_map::Range;

struct Subsystem {
    subsystem: Box<dyn AddressableIO>,
    address_range: Range<usize>,
    name: String,
}

impl Subsystem {
    pub fn new(
        name: &str,
        start_address: usize,
        subsystem: impl AddressableIO + 'static,
    ) -> Subsystem {
        let sub_len = subsystem.get_size();

        Subsystem {
            name: name.to_owned(),
            subsystem: Box::new(subsystem),
            address_range: Range {
                start: start_address,
                end: start_address + sub_len,
            },
        }
    }

    pub fn contains(&self, addr: usize) -> bool {
        self.address_range.contains(addr)
    }
}

impl AddressableIO for Subsystem {
    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        self.subsystem.read(addr, len)
    }

    fn write(&mut self, location: usize, data: &Vec<u8>) -> Result<(), MemoryError> {
        self.subsystem.write(location, data)
    }

    fn get_size(&self) -> usize {
        self.subsystem.get_size()
    }

    fn refresh(&mut self) {
        self.subsystem.refresh();
    }
}

impl fmt::Debug for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Subsystem {}, address range=#0x{:04X} → #0x{:04X}, size = {} bytes",
            self.name,
            self.address_range.start,
            self.address_range.end - 1,
            self.get_size()
        )
    }
}

#[derive(Debug)]
pub struct MemoryStack {
    stack: Vec<Subsystem>,
}

impl MemoryStack {
    pub fn new() -> MemoryStack {
        MemoryStack { stack: vec![] }
    }

    pub fn new_with_ram() -> MemoryStack {
        let mut memory_stack = Self::new();
        memory_stack.add_subsystem("RAM", 0x0000, RAM::new());

        memory_stack
    }

    pub fn add_subsystem(
        &mut self,
        name: &str,
        start_address: usize,
        memory: impl AddressableIO + 'static,
    ) {
        self.stack.push(Subsystem::new(name, start_address, memory));
    }

    pub fn get_subsystems_info(&self) -> Vec<String> {
        let mut output: Vec<String> = vec![];

        for sub in self.stack.iter() {
            output.push(format!("#{}: {:?}", output.len(), sub));
        }

        output
    }
}

impl AddressableIO for MemoryStack {
    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        let read_range: Range<usize> = Range::new(addr, addr + len);

        // iter on reverse order as our stack is LIFO
        for sub in self.stack.iter().rev() {
            // does our read range interfer with the current subsystem?
            if let Some(inter) = sub.address_range.intersection(&read_range) {
                // a touching range does not count, we deal with overlapping ranges
                if inter.start == inter.end {
                    continue;
                }
                // is the read range completely contained in the current subsystem?
                if inter == read_range {
                    return sub.read(addr - sub.address_range.start, len);
                } else {
                    // the read range runs accross several subsystems
                    // is this subsystem on the left or on the right of the read range?
                    if inter.start == read_range.start {
                        // we are on the left (the ending part of the current subsystem)
                        let mut left = sub.read(0, inter.end - addr)?;
                        let right = self.read(inter.end, len - (inter.end - addr))?;
                        left.extend_from_slice(right.as_slice());
                        return Ok(left);
                    } else {
                        // we are on the right (the starting part of the current subsystem)
                        let right = sub.read(0, inter.end - inter.start)?;
                        let mut left = self.read(addr, len - (inter.end - inter.start))?;
                        left.extend_from_slice(right.as_slice());
                        return Ok(left);
                    }
                }
            }
        }
        Err(MemoryError::Other(
            addr,
            "no memory subsystem at given location",
        ))
    }

    fn write(&mut self, addr: usize, data: &Vec<u8>) -> Result<(), MemoryError> {
        let write_range: Range<usize> = Range::new(addr, addr + data.len());
        for sub in self.stack.iter_mut().rev() {
            if let Some(inter) = sub.address_range.intersection(&write_range) {
                if inter.start == inter.end {
                    continue;
                }
                if inter == write_range {
                    return sub.write(addr - sub.address_range.start, data);
                } else if sub.address_range.contains(addr) {
                    // we are at the end of the current subsystem
                    let sub_start_addr = addr - sub.address_range.start;
                    let range_end = sub.address_range.end;
                    let split_addr = range_end - addr;
                    sub.write(sub_start_addr, &(data[..split_addr].to_vec()))?;
                    return self.write(range_end, &(data[split_addr..].to_vec()));
                } else {
                    // we are at the start of the current subsystem
                    let range_start = sub.address_range.start;
                    let split_addr = range_start - addr;
                    sub.write(0, &(data[split_addr..].to_vec()))?;
                    return self.write(addr, &(data[..split_addr].to_vec()));
                }
            }
        }
        Err(MemoryError::Other(addr, "no memory at given location"))
    }

    fn get_size(&self) -> usize {
        MEMMAX
    }

    fn refresh(&mut self) {
        for sub in self.stack.iter_mut() {
            sub.refresh();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_memory() -> MemoryStack {
        let mut memory_stack = MemoryStack::new();
        memory_stack.add_subsystem("RAM", 0x0000, RAM::new());
        memory_stack.add_subsystem("ROM", 0xC000, ROM::new([0xAE; 16384].to_vec()));

        memory_stack
    }

    #[test]
    fn test_add_subsystem() {
        let mut memory_stack = init_memory();
        let output = memory_stack.get_subsystems_info();
        assert_eq!(2, output.len());
        assert_eq!(
            "#0: Subsystem RAM, address range=#0x0000 → #0xFFFF, size = 65536 bytes",
            output[0]
        );
        assert_eq!(
            "#1: Subsystem ROM, address range=#0xC000 → #0xFFFF, size = 16384 bytes",
            output[1]
        );
    }

    #[test]
    fn test_read_one_subsystem() {
        let memory_stack = init_memory();
        let expected: Vec<u8> = vec![0x00, 0x00, 0x00, 0x00];
        assert_eq!(expected, memory_stack.read(0xAFFE, 4).unwrap());
        let expected: Vec<u8> = vec![0xae, 0xae, 0xae, 0xae];
        assert_eq!(expected, memory_stack.read(0xDFFE, 4).unwrap());
    }

    #[test]
    fn test_read_overlaping_subsystems() {
        let memory_stack = init_memory();
        let expected: Vec<u8> = vec![0x00, 0x00, 0xae, 0xae];
        assert_eq!(expected, memory_stack.read(0xBFFE, 4).unwrap());
        let expected: Vec<u8> = vec![0xae, 0xae];
        assert_eq!(expected, memory_stack.read(0xC000, 2).unwrap());
        let expected: Vec<u8> = vec![0x00, 0x00];
        assert_eq!(expected, memory_stack.read(0xBFFE, 2).unwrap());
    }

    #[test]
    fn test_write_one_subsystem() {
        let mut memory_stack = init_memory();
        let data: Vec<u8> = vec![0xff, 0xae, 0x81];
        memory_stack.write(0x1000, &data).unwrap();
        assert_eq!(
            vec![0xff, 0xae, 0x81],
            memory_stack.read(0x1000, 3).unwrap()
        );
    }

    #[test]
    fn test_write_overlapping_subsystems() {
        let mut memory_stack = init_memory();
        let data: Vec<u8> = vec![0xff, 0xae, 0x81];
        match memory_stack.write(0xBFFE, &data) {
            Err(MemoryError::Other(addr, msg)) => {
                assert_eq!(0x0000, addr);
                assert_eq!("trying to write in a read-only memory".to_owned(), msg);
            }
            v => panic!("it should return the expected error, got {:?}", v),
        };
    }
}

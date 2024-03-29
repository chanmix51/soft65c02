use super::*;
use range_map::Range;
use std::cmp;
use std::collections::BTreeMap;
use std::fmt::Debug;

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

    fn write(&mut self, location: usize, data: &[u8]) -> Result<(), MemoryError> {
        self.subsystem.write(location, data)
    }

    fn get_size(&self) -> usize {
        self.subsystem.get_size()
    }
}

impl fmt::Debug for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Subsystem {:<12}, address range=#0x{:04X} → #0x{:04X}, size = {} bytes",
            self.name,
            self.address_range.start,
            self.address_range.end - 1,
            self.get_size()
        )
    }
}

#[derive(Debug, Default)]
pub struct MemoryStack {
    stack: Vec<Subsystem>,
    address_map: BTreeMap<usize, usize>,
}

impl MemoryStack {
    pub fn new_with_ram() -> Self {
        let mut memory_stack = Self::default();
        memory_stack.add_subsystem("RAM", 0x0000, RAM::default());

        memory_stack
    }

    /*
     * The idea here is to ease the read & write operations.
     * An address_map is created to present only visible portions of subsystems' address range.
     * Once this is done, the only thing to do to read or write is to split the reads accross the
     * different subsystems.
     */
    pub fn add_subsystem(
        &mut self,
        name: &str,
        start_address: usize,
        memory: impl AddressableIO + 'static,
    ) {
        let end_address = start_address + memory.get_size();
        let sub = Subsystem::new(name, start_address, memory);
        let mut address_map: BTreeMap<usize, usize> = BTreeMap::new();
        address_map.insert(end_address, self.stack.len());
        let mut keys: Vec<usize> = vec![];
        self.address_map.keys().for_each(|x| keys.push(*x));

        if start_address != 0 {
            for (sub_index, sub) in self.stack.iter().enumerate() {
                if sub.contains(start_address - 1) {
                    address_map.insert(start_address, sub_index);
                    break;
                }
            }
        }

        for (&addr, &sub_index) in self.address_map.iter() {
            if !sub.address_range.contains(addr) {
                address_map.insert(addr, sub_index);
            }
        }
        self.address_map = address_map;
        self.stack.push(sub);
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
        let mut results: Vec<u8> = vec![];
        let mut tmplen = len;
        let mut tmpaddr = addr;
        for (&addr_split, &sub_index) in &self.address_map {
            if addr_split >= tmpaddr {
                let sublen = cmp::min(addr_split - tmpaddr, tmplen);
                let substart = self.stack[sub_index].address_range.start;
                if substart > tmpaddr {
                    return Err(MemoryError::Other(tmpaddr, "reading unallocated memory"));
                }
                let mut subr = self.stack[sub_index].read(tmpaddr - substart, sublen)?;
                results.append(&mut subr);
                tmplen -= sublen;
                tmpaddr += sublen;
            }
            if addr_split > addr + len {
                break;
            }
        }
        // there is still memory to read but no remaining subsystems
        if tmplen > 0 {
            return Err(MemoryError::ReadOverflow(tmplen, tmpaddr));
        }

        Ok(results)
    }

    fn write(&mut self, addr: usize, data: &[u8]) -> Result<(), MemoryError> {
        let len = data.len();
        let mut data = data.to_vec();
        let mut tmplen = len;
        let mut tmpaddr = addr;
        for (&addr_split, &sub_index) in &self.address_map {
            if addr_split > tmpaddr {
                let sublen = cmp::min(addr_split - tmpaddr, tmplen);
                let data_left = data.split_off(sublen);
                let substart = self.stack[sub_index].address_range.start;
                self.stack[sub_index].write(tmpaddr - substart, &data)?;
                data = data_left;
                tmplen -= sublen;
                tmpaddr += sublen;
            }
            if addr_split >= addr + len {
                break;
            }
        }

        Ok(())
    }

    fn get_size(&self) -> usize {
        MEMMAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeMemory {
        size: usize,
        content: u8,
    }

    impl FakeMemory {
        fn new(size: usize, content: u8) -> FakeMemory {
            FakeMemory { size, content }
        }
    }

    impl AddressableIO for FakeMemory {
        fn get_size(&self) -> usize {
            self.size
        }

        fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
            if addr + len > self.size {
                Err(MemoryError::ReadOverflow(len, addr))
            } else {
                Ok(vec![self.content; len])
            }
        }

        fn write(&mut self, addr: usize, data: &[u8]) -> Result<(), MemoryError> {
            if addr + data.len() > self.size {
                Err(MemoryError::WriteOverflow(data.len(), addr))
            } else {
                Ok(())
            }
        }
    }

    fn init_memory() -> MemoryStack {
        let mut memory_stack = MemoryStack::default();
        memory_stack.add_subsystem("RAM", 0x0000, RAM::default());
        memory_stack.add_subsystem("ROM", 0xC000, ROM::new([0xAE; 16384].to_vec()));

        memory_stack
    }

    #[test]
    fn test_add_subsystem() {
        let memory_stack = init_memory();
        let output = memory_stack.get_subsystems_info();
        assert_eq!(2, output.len());
        assert_eq!(
            "#0: Subsystem RAM         , address range=#0x0000 → #0xFFFF, size = 65536 bytes",
            output[0]
        );
        assert_eq!(
            "#1: Subsystem ROM         , address range=#0xC000 → #0xFFFF, size = 16384 bytes",
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

    #[test]
    fn test_write_over_entire_memory() {
        let mut memory_stack = MemoryStack::default();
        memory_stack.add_subsystem("RAM", 0x0000, RAM::default());
        memory_stack.add_subsystem("DUMMY", 0x8000, FakeMemory::new(1024, 0));
        let _ = memory_stack.read(0x7F00, 2048).unwrap();
        let data: Vec<u8> = vec![0xff; 2048];
        memory_stack.write(0x7F00, &data).unwrap();
    }

    #[test]
    #[should_panic]
    fn read_with_unallocated_memory() {
        let mut memory_stack = MemoryStack::default();
        memory_stack.add_subsystem("DUMMY", 0x0000, FakeMemory::new(0x1000, 0));
        memory_stack.add_subsystem("DUMMY", 0x2000, FakeMemory::new(0x1000, 1));
        match memory_stack.read(0x0000, 9 * 1024) {
            Err(MemoryError::Other(addr, msg)) => {
                assert_eq!(0x0000, addr);
                assert_eq!("trying to write in a read-only memory".to_owned(), msg);
            }
            v => panic!("it should return the expected error, got {:?}", v),
        }
    }

    #[test]
    fn read_over_memory() {
        let mut memory_stack = MemoryStack::default();
        memory_stack.add_subsystem("DUMMY", 0x0000, FakeMemory::new(0x1000, 0));
        match memory_stack.read(0, 0x2000) {
            Err(MemoryError::ReadOverflow(len, addr_start)) => {
                assert_eq!(0x1000, len);
                assert_eq!(0x1000, addr_start);
            }
            Ok(_) => panic!("this out of buffer read should not succeed"),
            _ => panic!("that was not the expected error"),
        }
    }
}

use fmt::Debug;
use range_map::Range;
use super::*;

struct Subsystem {
    subsystem: Box<dyn AddressableIO>,
    address_range: Range<usize>,
    name: String,
}

impl Subsystem {
    pub fn new(
        name: &str,
        start_address: usize,
        subsystem: impl AddressableIO + 'static
        ) -> Subsystem {
        let sub_len = subsystem.get_size();

        Subsystem {
            name: name.to_owned(),
            subsystem: Box::new(subsystem),
            address_range: Range {
                start: start_address,
                end: start_address + sub_len
                }
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

    fn write(&mut self, location: usize, data: Vec<u8>) -> Result<(), MemoryError> {
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
                "Subsystem '{}', address range=#{:?}, size = {} bytes",
                self.name,
                self.address_range,
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

    pub fn add_subsystem(&mut self, name: &str, start_address: usize, memory: impl AddressableIO + 'static) {
        self.stack.push(Subsystem::new(name, start_address, memory));
    }
}

impl AddressableIO for MemoryStack {
    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        let read_range: Range<usize> = Range::new(addr, addr + len);

        for sub in self.stack.iter().rev() {
            if let Some(inter) = sub.address_range.intersection(&read_range) {
                if inter == sub.address_range {
                    return sub.read(addr, len);
                } else {
                    let mut bytes = match sub.read(addr, inter.end - addr) {
                        Ok(v) => v,
                        e => return e,
                    };
                    match self.read(inter.end + 1, len - (inter.end + addr)) {
                        Ok(b) => {
                            bytes.extend_from_slice(b.as_slice());
                            return Ok(bytes)
                        },
                        e => return e,
                    }
                }
            }
        }
        Err(MemoryError::Other(addr, "no memory at given location"))
    }

    fn write(&mut self, addr: usize, data: Vec<u8>) -> Result<(), MemoryError> {
        let read_range: Range<usize> = Range::new(addr, addr + data.len());

        for sub in self.stack.iter_mut().rev() {
            if let Some(inter) = sub.address_range.intersection(&read_range) {
                return sub.write(addr, data);
            }
        }
        Err(MemoryError::Other(addr, "no memory at given location"))
    }

    fn get_size(&self) -> usize {
        MEMMAX
    }
}

#[cfg(tests)]
mod test {
    use super::*;

}

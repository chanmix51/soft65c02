mod memory;
mod registers;
mod addressing_mode;
mod cpu_instruction;
mod processing_unit;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

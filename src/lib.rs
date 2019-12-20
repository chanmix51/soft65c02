mod memory;
mod registers;
mod addressing_mode;
mod cpu_instruction;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

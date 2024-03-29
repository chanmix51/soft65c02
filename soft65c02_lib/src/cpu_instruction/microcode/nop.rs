use super::*;

pub fn nop(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(cpu_instruction, resolution, String::new()))
}

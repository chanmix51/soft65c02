## soft65c02_studio — Product Intent (WHAT)

### Purpose
Enable programmers to author, run, and iteratively refine `soft65c02_tester` scripts through an interactive terminal UI. The Studio streamlines taking exploratory debugging steps on 65C02 binaries and capturing those steps as executable test lines.

### Users
- Developers writing 65C02 machine code who want fast feedback and reproducible tests.

### Core Capabilities (MVP)
- Load a binary at a specified memory address.
- Create a new test script or load an existing one; save changes to disk (UTF‑8, Unix newlines).
- Execute the loaded test script inside the Studio and stream logs (including cycle counts and assertion results). After execution completes, return to the UI on keypress.
- Disassemble and execute code with a CP-centric execution view:
  - Show disassembly around the current CP: upcoming instructions (green), next instruction (bold), previous instructions (grey), sized to available terminal space.
  - Step, continue, run until a condition or address (e.g., CP=location).
- Memory window to view a region of memory.
- History/log of executed actions with cycle counts; from this history, select a single line to push into the test buffer.
- Test buffer and line editor:
  - Buffer is a list of script lines compatible with `soft65c02_tester` DSL (no Studio-only extensions).
  - Each line can be moved, deleted, duplicated.
  - Line editor can modify a loaded line and update the buffer.
  - Line editor can also import a command from history, edit it, rerun it, and/or push it into the buffer.
  - Comments and sections are supported in the buffer (saved as-is; no validation).
  
#### Command and mode model
- COMMAND mode: a top command line appears as an overlay with recent logs directly underneath; the main panes (disassembly, memory, test buffer) remain visible but are dimmed in the background. ESC keeps you in COMMAND; function keys may trigger commands (e.g., F1 → help).
- HELP mode: hides the command line and opens a centered modal window with help content (optional topic). ESC returns to COMMAND mode.
- EDIT mode: hides the command line and gives focus to the test line editor, where only tester-DSL lines are edited/entered. ESC returns to COMMAND mode.

### Explicit Constraints (MVP)
- Modify memory/registers: not supported in MVP (view-only).
- Persistence: only the test script is saved; history/session state are ephemeral.
- CPU: rely on `soft65c02_lib` for execution; support all 65C02 opcodes.
- Terminal UI: built with Ratatui; ANSI/UTF‑8 environment assumed.
- Disassembly: formatting consistent with tester’s verbose style where practical; number of upcoming lines depends on window size.
- DSL: 100% compatible with `soft65c02_tester`; scripts produced by Studio run unchanged in the tester.

### Inputs (MVP)
- Raw binary loaded at a user-provided address.
- Existing `soft65c02_tester` script (for viewing/editing/running).

### Outputs (MVP)
- Test script file saved to disk (UTF‑8, Unix newlines). No other artifacts exported.

### Primary User Flow (MVP)
1. Start Studio; create a new test file.
2. Load a binary at an address.
3. Verify the init vector; step the first three instructions.
4. From history, push the relevant lines into the test buffer at the selected position.
5. Save the test file.

### Studio Commands (MVP)
- File: `new`, `load <path>`, `save`, `quit`
- Execution: `run`, `loadbin <addr> <file>`, `reset init`, `reset <addr>`, `goto <addr>`
- Views: `disasm before=<n> after=<n>`, `mem base=<addr> len=<n>`
- Modes: `edit`, `help [topic]` (e.g., F1 maps to `help`)

### Non‑Goals (MVP)
- Editing memory/registers.
- Watchpoints, step over/out, run-to-cursor.
- Multi-line selection from history (single-line only in MVP).
- Syntax validation or linting of scripts (files saved as-is).
- Importing Atari XEX, Apple ProDOS AppleSingle, or symbol files (beyond raw binary at address).

### Future Enhancements (post‑MVP)
- Multi-line capture from history; range selection.
- Watchpoints, step over/out, run to cursor, reset via init vector.
- Edit memory/registers while paused.
- Import additional binary formats (Atari XEX, AppleSingle), and symbol files (VICE format) with symbol-aware disassembly.
- Export disassembly snapshots and/or memory dumps.
- Optional script validation prior to save.



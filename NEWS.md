## lumesh Recent Updates (0.14.x → 0.16.x)

### Complete Editor Rewrite (0.15.0)

Version 0.15.0 was a major milestone: **migrated from rustyline to crossterm, editor rewritten from scratch**. Subsequent versions continued refinement:

- **0.15.1**: Editor themes, buffer optimization, custom hotkey bindings
- **0.15.2**: Multi-line editing mode, input validation, cursor and hint position fixes
- **0.16.0**: Fixed ONLCR issue, path completion sorting optimization
- **0.16.2**: Added `ui.editor`, `ui.date_pick`, fixed CapsLock recognition, hotkey modifier mapping, and other bugs
- **0.16.10**: Editor scrolling support

---

### Tokenizer Refactoring (0.15.5 onwards)

- **0.15.5**: **Restructured tokenizer with dispatch mechanism**, dispatching token parsing by priority for clearer and more maintainable parsing logic
- **0.15.6**: Improved CFM (Command First Mode) symbol handling, unified highlighting logic
- **0.15.7**: Fixed tokenizer handling of trailing `&`
- **0.16.2**: Fixed module call tokenizer
- **0.16.7**: CFM mode enhanced to take whole words (avoiding misparsing `1.1.1.1` as float)
- **0.16.8**: Switched to static regex for improved tokenization performance

---

### History System Evolution

- **0.16.5**: Added history hint, ESC moves to end in multiline mode and clears hint
- **0.16.8**: **Introduced slash commands** (`/h`, `/hh`, `/hm` and other slash command system)
- **0.16.9**: Smarter history weighting and sorting, added `/q` quick exit command
- **0.16.10**:
  - Optimized `Ctrl+R` long history display
  - Multi-line commands automatically ignored from `/h...` history (avoiding screen clutter)
  - `/h`, `/hh`, `/hm`, `/history` support prefix filtering

---

### Completion System Enhancements (0.15.3–0.16.3)

- Path completion, external command completion, parameter-aware completion progressively improved
- Support for lumesh scripts as completion data sources
- Completion colors and context awareness
- `ui.pick`/`ui.multi_pick` support `table`/`map` type input
- `ui.float` supports custom decimal places

---

### AI Integration Deepening

- **0.15.9**: `ALT+i` triggers AI hint
- **0.16.0**: `ALT+Enter` / `ALT+o` triggers AI generation
- **0.16.3**: ai-tls enabled by default, ai-https separated as optional feature
- **0.16.8**: Updated AI skill configuration
- **0.16.10**: Updated AI documentation

---

### Language Features

- **0.14.0**: Introduced `table` expression and built-in `table` library
- **0.14.3**: Improved quote semantics (`''` raw string / `""` normal escape / ` `` ` full escape + variable interpolation)
- **0.16.5**: Added `continue` statement; `match` arrow supports line breaks; **Breaking change: only allow single value in declarations**
- **0.16.7**: `~` auto-expands in symbols, normal mode supports prefix matching

---

### Standard Library Restructuring (0.16.10)

Module responsibilities reorganized:

| Change | Description |
|--------|-------------|
| `fs.dirs` → `sys` | Directory-related functions moved to system library |
| `sys.print_tty` / `sys.discard` → `console` | Terminal output control moved to console library |
| Delete `sys.cds` | Cleaned up redundant interfaces |
| Fix float file size display | `float filesize` formatting fix |

---

### Phase Summary

| Phase | Core Direction |
|-------|----------------|
| 0.14.x | Language feature refinement (table, quotes, bug fixes) |
| 0.15.x | Editor rewrite + tokenizer refactoring + completion system |
| 0.16.x | Slash commands system + History intelligence + AI integration + module reorganization |

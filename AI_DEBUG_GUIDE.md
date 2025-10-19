# AI_DEBUG_GUIDE.md
> **Purpose:**  
> This document explains how Codex CLI (or any AI agent) can debug and fix issues in this Rust + Slint GUI application.

---

## 🧩 Overview

This project is a **native GUI text editor** built in **Rust + Slint**.  
Since GUI applications do not expose their state through CLI by default, this guide describes a standardized **AI debugging protocol** that allows a CLI-based AI agent to:

- Run the GUI in test mode (`--test`)
- Observe and log UI state changes in JSON format
- Optionally capture screenshots for visual verification
- Analyze logs and suggest or apply code fixes

---

## ⚙️ 1. Test Execution Mode

### 🔧 Command
```bash
cargo run -- --test
```

### 🧠 Description
Running with the `--test` flag triggers a non-interactive test mode in which:
- The app simulates basic UI interactions (button clicks, state changes)
- Internal state is dumped to `debug_state.json`
- Any errors or unexpected values are printed to stdout and `debug.log`

This allows Codex CLI to run the app **headlessly**, without a physical display.

---

## 🧾 2. State Observation

During runtime, all meaningful UI updates and interactions are serialized to `debug_state.json`.

Example format:

```json
{
  "timestamp": "2025-10-18T12:30:00Z",
  "text_buffer": "Hello World",
  "cursor_position": 5,
  "window_title": "Altre Editor",
  "error": null
}
```

### ✅ Codex CLI Task
- Parse `debug_state.json`
- Detect abnormal states (e.g., wrong values, missing updates, panic traces)
- Suggest or apply code corrections in relevant Rust files

---

## 📸 3. Screenshot Capture (optional visual check)

If running in CI or virtualized environments (Linux only):

```bash
xvfb-run --server-num=1 ./target/debug/my_editor --test &
sleep 1
import -window root screenshot.png
```

The file `screenshot.png` can then be analyzed by Codex CLI (if image analysis is supported).

Use this for:
- Layout or rendering issues
- Missing UI components
- Text misalignment or visual artifacts

---

## 🧰 4. Event and Error Logging

All user interactions or simulated events are recorded in `debug.log`.  
Each entry follows this format:

```
[2025-10-18T12:31:12Z] EVENT: ButtonClicked -> counter = 4
[2025-10-18T12:31:13Z] STATE: { "counter": 4, "text": "Hello" }
[2025-10-18T12:31:14Z] ERROR: Cannot render label "file_status"
```

### ✅ Codex CLI Task
- Read and parse `debug.log`
- Identify error lines or unusual event sequences
- Suggest fixes to relevant handlers (e.g. `.on_button_clicked`, `.set_property`, etc.)

---

## 🔗 5. HTTP Observation API (optional)

When compiled with `--features debug_api`, the app exposes a local debug API:

```bash
cargo run --features debug_api
```

### Endpoint
```
GET http://localhost:7070/state
```

### Response Example
```json
{
  "buffer": "fn main() {}",
  "cursor": { "line": 3, "column": 1 },
  "status": "idle"
}
```

### ✅ Codex CLI Task
- Perform HTTP GET request to `/state`
- Compare the returned state with expected behavior
- Suggest or apply code fixes if inconsistency is detected

---

## 🧪 6. AI Debug Workflow Summary

| Step | Action | CLI Command | Output |
|------|---------|--------------|---------|
| 1 | Build & run test mode | `cargo run -- --test` | `debug_state.json`, `debug.log` |
| 2 | Analyze state | `cat debug_state.json` | JSON |
| 3 | Analyze log | `cat debug.log` | Text |
| 4 | (Optional) Capture screenshot | `import -window root screenshot.png` | PNG |
| 5 | Fix issues | AI modifies Rust code | Commit & rerun |

---

## 🧩 7. Key Principles for Codex CLI

1. **Never expect direct GUI feedback.**  
   Use text logs (`debug.log`) and state JSONs (`debug_state.json`).

2. **Use `cargo run -- --test` or `cargo test` for execution.**

3. **After modification, always rerun the test** and verify that:
   - No new errors appear in logs
   - State transitions are consistent

4. **When in doubt**, print additional logs or serialize more internal data.

---

## 🧩 8. Example Commands for AI

```bash
# Run and collect logs
cargo run -- --test > output.log 2>&1

# Inspect current UI state
cat debug_state.json

# Look for runtime errors
grep ERROR debug.log

# Rebuild after fix
cargo build
```

---

## 🧭 9. Common Error Categories (AI should detect these)

| Type | Symptom | Typical Cause | Fix Strategy |
|------|----------|----------------|---------------|
| Runtime Panic | App exits immediately | Borrow/unwrap misuse | Add Option handling |
| UI Freeze | No state change | Missing `invoke_*` call | Trigger event explicitly |
| State Desync | JSON shows old data | Not calling `set_property()` | Add state sync |
| Render Error | Label/button missing | `.slint` mismatch | Adjust `.slint` file |
| Text corruption | Garbled output | Unicode mishandling | Use `.into()` or `.clone()` on `SharedString` |

---

## 🧩 10. Integration Hints

Codex CLI should:
- Treat this file (`AI_DEBUG_GUIDE.md`) as the debugging protocol
- Use test commands from section 6
- Never attempt direct GUI interaction
- Always provide reasoning when proposing code changes

---

> **Maintainer Note:**  
> This guide is intentionally AI-readable.  
> Any CLI agent capable of file access, command execution, and JSON parsing can use it to debug this Slint-based GUI project.

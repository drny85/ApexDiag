//! ELM327 / STN11xx (OBDLink) AT-command pipeline and response parser.
//!
//! Step 2 adds the full response parser (headers on, multi-line frames,
//! `NO DATA`, `CAN ERROR`, prompt handling).

/// The init sequence sent after the link opens, in order.
pub const INIT_SEQUENCE: &[&str] = &[
    "AT Z",    // reset
    "AT E0",   // echo off
    "AT L0",   // linefeeds off
    "AT S1",   // spaces on (easier parsing while developing)
    "AT H1",   // headers on
    "AT SP 6", // ISO 15765-4, 11-bit, 500 kbaud
];

/// Extra commands used when we run our own ISO-TP over raw CAN.
pub const RAW_CAN_MODE: &[&str] = &["AT CAF 0"];

/// Format a command with the trailing carriage return the adapter expects.
pub fn command(cmd: &str) -> String {
    format!("{cmd}\r")
}

/// True when the adapter has finished replying (it prints `>` when ready).
pub fn is_prompt(buf: &str) -> bool {
    buf.trim_end().ends_with('>')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_starts_with_reset() {
        assert_eq!(INIT_SEQUENCE[0], "AT Z");
        assert_eq!(command("AT E0"), "AT E0\r");
    }

    #[test]
    fn detects_prompt() {
        assert!(is_prompt("OK\r\r>"));
        assert!(!is_prompt("SEARCHING..."));
    }
}

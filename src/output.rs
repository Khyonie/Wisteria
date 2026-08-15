use std::{
    env,
    io::{self, IsTerminal, Write},
};

use serde_json::json;

const STATUS_STYLE: &str = "\x1b[1;7m";
const RESET_STYLE: &str = "\x1b[0m";
const DEFAULT_TERMINAL_WIDTH: usize = 80;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputMode {
    #[default]
    Auto,
    Plain,
    Terminal,
    Json,
}

impl OutputMode {
    pub fn load(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "plain" | "text" => Ok(Self::Plain),
            "terminal" | "tty" => Ok(Self::Terminal),
            "json" | "ndjson" => Ok(Self::Json),
            _ => Err(format!(
                "Unknown output mode \"{value}\". Expected one of [auto, plain, terminal, json]."
            )),
        }
    }
}

pub fn renderer(mode: OutputMode) -> Box<dyn OutputRenderer> {
    match mode {
        OutputMode::Auto if io::stdout().is_terminal() => Box::new(TerminalOutput::new()),
        OutputMode::Auto | OutputMode::Plain => Box::new(PlainOutput),
        OutputMode::Terminal => Box::new(TerminalOutput::new()),
        OutputMode::Json => Box::new(JsonOutput),
    }
}

pub trait OutputRenderer {
    fn operation_started(&mut self, operation: &str, total_steps: usize);
    fn operation_completed(&mut self, operation: &str, message: &str);
    fn step_started(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
    );
    fn step_completed(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
        message: &str,
    );
    fn step_failed(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
        message: &str,
    );
    fn log(&mut self, message: &str);

    fn suspend(&mut self) {}
}

pub fn log_process_output(renderer: &mut dyn OutputRenderer, stdout: &[u8], stderr: &[u8]) {
    log_process_stream(renderer, stdout);
    log_process_stream(renderer, stderr);
}

fn log_process_stream(renderer: &mut dyn OutputRenderer, bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }

    let message = String::from_utf8_lossy(bytes);
    let message = message.trim_end_matches(['\r', '\n']);
    if !message.is_empty() {
        renderer.log(message);
    }
}

struct PlainOutput;

impl OutputRenderer for PlainOutput {
    fn operation_started(&mut self, _operation: &str, _total_steps: usize) {
        // Plain output stays append-only and avoids extra wrapper lines around existing commands.
    }

    fn operation_completed(&mut self, _operation: &str, message: &str) {
        if !message.is_empty() {
            println!("{message}");
        }
    }

    fn step_started(
        &mut self,
        _operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
    ) {
        println!("({index}/{total}) {action} {item}");
    }

    fn step_completed(
        &mut self,
        _operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
        message: &str,
    ) {
        println!("({index}/{total}) {action} {item}: {message}");
    }

    fn step_failed(
        &mut self,
        _operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
        message: &str,
    ) {
        println!("({index}/{total}) {action} {item}: Failed");
        println!("{message}");
    }

    fn log(&mut self, message: &str) {
        println!("{message}");
    }
}

struct JsonOutput;

impl OutputRenderer for JsonOutput {
    fn operation_started(&mut self, operation: &str, total_steps: usize) {
        println!(
            "{}",
            json!({
                "type": "operation_started",
                "operation": operation,
                "total_steps": total_steps,
            })
        );
    }

    fn operation_completed(&mut self, operation: &str, message: &str) {
        println!(
            "{}",
            json!({
                "type": "operation_completed",
                "operation": operation,
                "message": message,
            })
        );
    }

    fn step_started(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
    ) {
        println!(
            "{}",
            json!({
                "type": "step_started",
                "operation": operation,
                "action": action,
                "item": item,
                "index": index,
                "total": total,
            })
        );
    }

    fn step_completed(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
        message: &str,
    ) {
        println!(
            "{}",
            json!({
                "type": "step_completed",
                "operation": operation,
                "action": action,
                "item": item,
                "index": index,
                "total": total,
                "message": message,
            })
        );
    }

    fn step_failed(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
        message: &str,
    ) {
        println!(
            "{}",
            json!({
                "type": "step_failed",
                "operation": operation,
                "action": action,
                "item": item,
                "index": index,
                "total": total,
                "message": message,
            })
        );
    }

    fn log(&mut self, message: &str) {
        println!(
            "{}",
            json!({
                "type": "log",
                "message": message,
            })
        );
    }
}

struct TerminalOutput {
    live_lines: usize,
    operation: Option<String>,
    total_steps: usize,
    active_line: Option<String>,
    status_line: Option<String>,
    spinner_index: usize,
}

impl TerminalOutput {
    fn new() -> Self {
        Self {
            live_lines: 0,
            operation: None,
            total_steps: 0,
            active_line: None,
            status_line: None,
            spinner_index: 0,
        }
    }

    fn redraw(&mut self) {
        self.clear_live_region();
        let line_width = live_line_width();

        let mut live_lines = 0;
        if let Some(active_line) = &self.active_line {
            println!("{}", truncate_visible_line(active_line, line_width));
            live_lines += 1;
        }

        if let Some(status_line) = &self.status_line {
            print!("{}", format_status_line(status_line, line_width));
            live_lines += 1;
        }

        let _ = io::stdout().flush();
        self.live_lines = live_lines;
    }

    fn clear_live_region(&mut self) {
        if self.live_lines == 0 {
            return;
        }

        print!("\r");
        for line in 0..self.live_lines {
            print!("\x1b[2K");
            if line + 1 < self.live_lines {
                print!("\x1b[1A\r");
            }
        }

        self.live_lines = 0;
    }

    fn log_above_status(&mut self, message: &str) {
        self.clear_live_region();
        println!("{message}");
        if self.status_line.is_some() || self.active_line.is_some() {
            self.redraw();
        }
    }

    fn update_status(&mut self, operation: &str, index: usize, total: usize) {
        let spinner = self.next_spinner();
        self.status_line = Some(format_status_content(spinner, operation, index, total));
    }

    fn next_spinner(&mut self) -> &'static str {
        const FRAMES: [&str; 4] = ["-", "\\", "|", "/"];
        let frame = FRAMES[self.spinner_index % FRAMES.len()];
        self.spinner_index += 1;
        frame
    }
}

fn format_status_content(spinner: &str, operation: &str, index: usize, total: usize) -> String {
    format!(" {spinner} STATUS running \"{operation}\" ({index}/{total}) ")
}

fn format_status_line(status_line: &str, width: usize) -> String {
    format!(
        "{STATUS_STYLE}{}{RESET_STYLE}",
        truncate_visible_line(status_line, width)
    )
}

fn live_line_width() -> usize {
    terminal_width().saturating_sub(1).max(1)
}

fn terminal_width() -> usize {
    env::var("COLUMNS")
        .ok()
        .and_then(|columns| columns.parse::<usize>().ok())
        .filter(|columns| *columns > 0)
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
}

fn truncate_visible_line(line: &str, width: usize) -> String {
    let length = line.chars().count();
    if length <= width {
        return line.to_string();
    }

    if width <= 3 {
        return line.chars().take(width).collect();
    }

    let mut truncated: String = line.chars().take(width - 3).collect();
    truncated.push_str("...");
    truncated
}

fn is_meaningful_step_message(message: &str) -> bool {
    !matches!(message, "" | "Done")
}

fn format_step_completion(action: &str, item: &str, message: &str) -> String {
    match action {
        "Collecting" => format!("Collected {message}"),
        "Compiling" => format!("Compiled {message}"),
        "Packaging" => format!("Packaged {item}: {message}"),
        "Shading" if message.starts_with("No ") => message.to_string(),
        _ => format!("{action} {item}: {message}"),
    }
}

impl OutputRenderer for TerminalOutput {
    fn operation_started(&mut self, operation: &str, total_steps: usize) {
        self.operation = Some(operation.to_string());
        self.total_steps = total_steps;
        self.active_line = None;
        let spinner = self.next_spinner();
        self.status_line = Some(format_status_content(spinner, operation, 0, total_steps));
        self.redraw();
    }

    fn operation_completed(&mut self, _operation: &str, message: &str) {
        self.clear_live_region();
        self.operation = None;
        self.active_line = None;
        self.status_line = None;
        if !message.is_empty() {
            println!("{message}");
        }
    }

    fn step_started(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
    ) {
        self.active_line = Some(format!("{action} {item}"));
        self.update_status(operation, index, total);
        self.redraw();
    }

    fn step_completed(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
        message: &str,
    ) {
        self.update_status(operation, index, total);
        if is_meaningful_step_message(message) {
            self.active_line = Some(format_step_completion(action, item, message));
        }
        self.redraw();
    }

    fn step_failed(
        &mut self,
        operation: &str,
        action: &str,
        item: &str,
        index: usize,
        total: usize,
        message: &str,
    ) {
        self.update_status(operation, index, total);
        self.log_above_status(&format!("{action} {item}: Failed"));
        self.log_above_status(message);
    }

    fn log(&mut self, message: &str) {
        self.log_above_status(message);
    }

    fn suspend(&mut self) {
        self.clear_live_region();
        let _ = io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_mode_loads_known_values() {
        assert_eq!(OutputMode::load("auto"), Ok(OutputMode::Auto));
        assert_eq!(OutputMode::load("plain"), Ok(OutputMode::Plain));
        assert_eq!(OutputMode::load("text"), Ok(OutputMode::Plain));
        assert_eq!(OutputMode::load("terminal"), Ok(OutputMode::Terminal));
        assert_eq!(OutputMode::load("tty"), Ok(OutputMode::Terminal));
        assert_eq!(OutputMode::load("json"), Ok(OutputMode::Json));
        assert_eq!(OutputMode::load("ndjson"), Ok(OutputMode::Json));
    }

    #[test]
    fn output_mode_rejects_unknown_values() {
        let error = OutputMode::load("xml").unwrap_err();

        assert!(error.contains("Unknown output mode"));
        assert!(error.contains("auto, plain, terminal, json"));
    }

    #[test]
    fn terminal_status_line_is_visually_distinct() {
        let status_line = format_status_line(&format_status_content("-", "build", 1, 5), 80);

        assert!(status_line.starts_with(STATUS_STYLE));
        assert!(status_line.ends_with(RESET_STYLE));
        assert!(status_line.contains("STATUS running \"build\" (1/5)"));
    }

    #[test]
    fn terminal_live_lines_are_truncated_to_one_row() {
        assert_eq!(
            truncate_visible_line("Resolving dependency", 11),
            "Resolvin..."
        );
        assert_eq!(truncate_visible_line("abc", 2), "ab");
    }

    #[test]
    fn terminal_step_completion_summarizes_common_build_steps() {
        assert_eq!(
            format_step_completion("Collecting", "sources", "3 source files"),
            "Collected 3 source files"
        );
        assert_eq!(
            format_step_completion("Compiling", "classes", "1 source file"),
            "Compiled 1 source file"
        );
    }
}

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use console::{measure_text_width, strip_ansi_codes};
use forge_api::Environment;
use nu_ansi_term::Style;
use rustyline::completion::{Completer, Pair};
use rustyline::config::{ColorMode, CompletionType, Config};
use rustyline::error::ReadlineError as RustyReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{
    Cmd, Context as RustylineContext, Editor, EventHandler, Helper, KeyCode, KeyEvent, Modifiers,
    Prompt as RustylinePrompt,
};

use super::completer::InputCompleter;
use super::zsh::paste::wrap_pasted_text;
use crate::highlighter::ForgeHighlighter;
use crate::model::ForgeCommandManager;
use crate::prompt::ForgePrompt;

const HISTORY_CAPACITY: usize = 1024 * 1024;

/// Interactive terminal editor used by the Forge prompt.
pub struct ForgeEditor {
    editor: Editor<ForgeHelper, DefaultHistory>,
    history_file: PathBuf,
    pending_buffer: Option<String>,
}

/// Result of reading one prompt interaction from the terminal.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadResult {
    Success(String),
    Empty,
    Continue,
    Exit,
}

impl ForgeEditor {
    /// Creates a new interactive editor with history, completion, and
    /// highlighting.
    pub fn new(
        env: Environment,
        custom_history_path: Option<PathBuf>,
        manager: Arc<ForgeCommandManager>,
    ) -> Self {
        let history_file = env.history_path(custom_history_path.as_ref());
        let helper = ForgeHelper::new(env.cwd, manager);
        let config = Config::builder()
            .max_history_size(HISTORY_CAPACITY)
            .expect("rustyline history capacity should be valid")
            .completion_type(CompletionType::List)
            .completion_show_all_if_ambiguous(true)
            .color_mode(ColorMode::Forced)
            .enable_signals(true)
            .build();
        let mut editor = Editor::<ForgeHelper, DefaultHistory>::with_config(config)
            .expect("rustyline editor should initialize for an interactive terminal");
        editor.bind_sequence(
            KeyEvent(KeyCode::Enter, Modifiers::ALT),
            EventHandler::Simple(Cmd::Newline),
        );
        editor.bind_sequence(
            KeyEvent(KeyCode::Char('k'), Modifiers::CTRL),
            EventHandler::Simple(Cmd::ClearScreen),
        );
        editor.bind_sequence(
            KeyEvent(KeyCode::Char('K'), Modifiers::CTRL),
            EventHandler::Simple(Cmd::ClearScreen),
        );
        editor.set_helper(Some(helper));
        let _ = editor.load_history(&history_file);
        Self { editor, history_file, pending_buffer: None }
    }

    fn normalize_result(&mut self, buffer: String) -> ReadResult {
        let result = normalize_result_text(buffer);
        if let ReadResult::Success(text) = &result {
            let _ = self.editor.add_history_entry(text.as_str());
            let _ = self.editor.save_history(&self.history_file);
        }
        result
    }

    /// Reads one logical input from the terminal.
    pub fn prompt(&mut self, prompt: &mut ForgePrompt) -> anyhow::Result<ReadResult> {
        let prompt_text = render_prompt(prompt);
        let initial = self.pending_buffer.take().unwrap_or_default();
        let readline = if initial.is_empty() {
            self.editor.readline(&prompt_text)
        } else {
            self.editor
                .readline_with_initial(&prompt_text, (&initial, ""))
        };
        prompt.refresh();

        match readline {
            Ok(buffer) => Ok(self.normalize_result(buffer)),
            Err(RustyReadlineError::Interrupted) => Ok(ReadResult::Continue),
            Err(RustyReadlineError::Eof) => Ok(ReadResult::Exit),
            Err(error) => Err(anyhow::anyhow!(ReadLineError(error))),
        }
    }

    /// Sets the buffer content to be pre-filled on the next prompt.
    pub fn set_buffer(&mut self, content: String) {
        self.pending_buffer = Some(content);
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to read line from terminal: {0}")]
pub struct ReadLineError(RustyReadlineError);

fn normalize_result_text(buffer: String) -> ReadResult {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return ReadResult::Empty;
    }
    ReadResult::Success(wrap_pasted_text(trimmed))
}

fn render_prompt(prompt: &ForgePrompt) -> ResponsivePrompt {
    let left = prompt.render_prompt_left();
    let indicator = prompt.render_prompt_indicator();
    let right = prompt.render_prompt_right();
    let right = right.trim();

    // The left prompt is multi-line: the first line carries the dir/branch and
    // is where the right prompt is aligned, while the editable input sits after
    // the chevron on the last line. Keep `raw` (rustyline's width/cursor model)
    // as the left prompt only — the right prompt is padded onto the first line
    // in `styled`, which is not the line the cursor rests on, so the layout
    // model stays correct on every platform.
    if let Some((first_line, remaining)) = left.split_once('\n') {
        let raw = raw_prompt(&format!("{first_line}\n{remaining}{indicator}"));
        let styled = match align_right(first_line, right) {
            Some(aligned) => format!("{first_line}{aligned}\n{remaining}{indicator}"),
            None => format!("{first_line}\n{remaining}{indicator}"),
        };
        return ResponsivePrompt { raw, styled };
    }

    // Degenerate single-line prompt: there is no separate line to host a right
    // prompt without colliding with the input, so render the left prompt only.
    let styled = format!("{left}{indicator}");
    ResponsivePrompt { raw: raw_prompt(&styled), styled }
}

/// Builds the raw (layout) form of a prompt by stripping every ANSI escape
/// sequence from its styled form.
///
/// rustyline's [`Prompt::raw`] contract requires a string with no style and no
/// ANSI escape sequence: it is the reference used to compute the prompt's
/// display width and cursor placement. Embedding color codes here makes
/// Windows consoles count the escape bytes as visible columns, producing a
/// large spurious indent before the editor. Stripping the codes keeps the raw
/// width correct on every platform while the styled form retains the colors.
fn raw_prompt(styled: &str) -> String {
    strip_ansi_codes(styled).into_owned()
}

/// Right-aligns the styled `right` prompt on the same line as `left_line` using
/// plain padding spaces.
///
/// Earlier versions positioned the right prompt with absolute cursor escapes
/// (save `\x1b[s`, jump to the far right `\x1b[999C`, restore `\x1b[u`). Those
/// are honored inconsistently by Windows consoles and IDE-embedded terminals,
/// where they desynced rustyline's cursor model and made the whole first prompt
/// line duplicate and slide right on every redraw. Padding with spaces moves no
/// cursor and keeps `raw`/`styled` in sync everywhere.
///
/// Returns `None` when the terminal width is unknown or too narrow to fit both
/// the left content and the right prompt with at least one space of separation
/// and a one-column trailing margin (so the line never reaches the right edge
/// and wraps); the caller then omits the right prompt.
fn align_right(left_line: &str, right: &str) -> Option<String> {
    let cols = terminal_size::terminal_size().map(|(w, _)| w.0 as usize)?;
    align_right_in(left_line, right, cols)
}

/// Right-aligns `right` after `left_line` within a `cols`-wide terminal.
///
/// Split out from [`align_right`] so the padding math is testable without a
/// real terminal. Returns `None` when `right` is empty or the two cannot
/// coexist on one line with a separating space and a one-column trailing
/// margin.
fn align_right_in(left_line: &str, right: &str, cols: usize) -> Option<String> {
    if right.is_empty() {
        return None;
    }
    let left_w = measure_text_width(strip_ansi_codes(left_line).as_ref());
    let right_w = measure_text_width(strip_ansi_codes(right).as_ref());
    // Reserve one trailing column as a margin so a slight width miscount of the
    // nerd-font glyphs cannot push the line past the right edge into a wrap.
    let margin = 1;
    // Require at least one space of separation between left and right content.
    if cols < left_w + right_w + margin + 1 {
        return None;
    }
    let pad = cols - left_w - right_w - margin;
    Some(format!("{}{right}", " ".repeat(pad)))
}

struct ResponsivePrompt {
    raw: String,
    styled: String,
}

impl RustylinePrompt for ResponsivePrompt {
    fn raw(&self) -> &str {
        &self.raw
    }

    fn styled(&self) -> &str {
        &self.styled
    }
}

struct ForgeHelper {
    completer: Mutex<InputCompleter>,
    highlighter: ForgeHighlighter,
    hinter: HistoryHinter,
}

impl ForgeHelper {
    fn new(cwd: PathBuf, command_manager: Arc<ForgeCommandManager>) -> Self {
        Self {
            completer: Mutex::new(InputCompleter::new(cwd, command_manager)),
            highlighter: ForgeHighlighter,
            hinter: HistoryHinter {},
        }
    }
}

impl Helper for ForgeHelper {}

impl Completer for ForgeHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &RustylineContext<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut completer = self
            .completer
            .lock()
            .expect("input completer mutex poisoned");
        let suggestions = completer.complete(line, pos);
        let start = suggestions
            .iter()
            .map(|suggestion| suggestion.span.start)
            .min()
            .unwrap_or(pos);
        let pairs = suggestions
            .into_iter()
            .map(|suggestion| {
                let replacement = if suggestion.append_whitespace {
                    format!("{} ", suggestion.value)
                } else {
                    suggestion.value
                };
                Pair { display: replacement.clone(), replacement }
            })
            .collect();
        Ok((start, pairs))
    }
}

impl Hinter for ForgeHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &RustylineContext<'_>) -> Option<Self::Hint> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for ForgeHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        let styled = self.highlighter.highlight(line, pos);
        if styled.buffer.is_empty() {
            return Cow::Borrowed(line);
        }

        let default_style = Style::new();
        let mut rendered = String::with_capacity(line.len());
        for (style, text) in styled.buffer {
            if style == default_style {
                rendered.push_str(&text);
            } else {
                rendered.push_str(&style.paint(text).to_string());
            }
        }
        Cow::Owned(rendered)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(Style::new().dimmed().paint(hint).to_string())
    }
}

impl Validator for ForgeHelper {}

#[cfg(test)]
mod tests {
    use forge_api::AgentId;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_normalize_result_wraps_existing_pasted_path() {
        let fixture = "/usr/bin/env".to_string();

        let actual = normalize_result_text(fixture);

        let expected = ReadResult::Success("@[/usr/bin/env]".to_string());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_raw_prompt_strips_ansi_escape_sequences() {
        let fixture = "\x1b[1m\x1b[92m\u{f013e}\x1b[0m test";

        let actual = raw_prompt(fixture);

        let expected = "\u{f013e} test".to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_render_prompt_raw_has_no_ansi_but_styled_keeps_color() {
        let fixture = ForgePrompt {
            cwd: PathBuf::from("/tmp/project"),
            usage: None,
            agent_id: AgentId::default(),
            model: None,
            reasoning_effort: None,
            git_branch: Some("main".to_string()),
        };

        let actual = render_prompt(&fixture);

        // rustyline uses `raw` for width/cursor layout: it must be free of ANSI
        // escape sequences, otherwise Windows consoles count the escape bytes as
        // visible columns and indent the editor. `styled` keeps the colors.
        assert!(
            !actual.raw.contains('\x1b'),
            "raw prompt must not contain ANSI escapes, got: {:?}",
            actual.raw
        );
        assert!(
            actual.styled.contains('\x1b'),
            "styled prompt should retain ANSI color codes"
        );
    }

    #[test]
    fn test_align_right_in_pads_to_right_edge() {
        // Plain left + right within a wide terminal: padding fills the gap and
        // leaves a one-column trailing margin (so total width == cols - 1).
        let actual = align_right_in("left", "RIGHT", 20);

        let expected = Some(format!("{}RIGHT", " ".repeat(20 - 4 - 5 - 1)));
        assert_eq!(actual, expected);
        // Visible width never reaches the terminal edge (avoids a wrap).
        let line = format!("left{}", actual.unwrap());
        assert_eq!(line.chars().count(), 20 - 1);
    }

    #[test]
    fn test_align_right_in_uses_no_cursor_escapes() {
        // Regression: the right prompt must be padded with spaces only, never
        // absolute cursor moves (\x1b[s / \x1b[999C / \x1b[u), which slid and
        // duplicated the prompt on Windows/IDE terminals.
        let actual = align_right_in("left", "RIGHT", 40).unwrap();

        assert!(!actual.contains('\x1b'));
        assert!(!actual.contains("999C"));
    }

    #[test]
    fn test_align_right_in_too_narrow_returns_none() {
        // No room for both with a separating space + margin → omit right prompt.
        let actual = align_right_in("aaaaaa", "bbbbb", 10);

        assert_eq!(actual, None);
    }

    #[test]
    fn test_align_right_in_empty_right_returns_none() {
        let actual = align_right_in("left", "", 80);

        assert_eq!(actual, None);
    }
}

use rustyline::completion::{Candidate, Completer, FilenameCompleter, Pair};
use rustyline::line_buffer;

use termion::event::Key;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Paragraph, Text};

use crate::context::JoshutoContext;
use crate::ui::TuiBackend;
use crate::util::event::Event;

use super::{TuiMenu, TuiView};

struct CompletionTracker {
    pub index: usize,
    pub pos: usize,
    pub original: String,
    pub candidates: Vec<Pair>,
}

impl CompletionTracker {
    pub fn new(pos: usize, candidates: Vec<Pair>, original: String) -> Self {
        CompletionTracker {
            index: 0,
            pos,
            original,
            candidates,
        }
    }
}

pub struct TuiTextField<'a> {
    _prompt: &'a str,
    _prefix: &'a str,
    _suffix: &'a str,
    _menu: Option<TuiMenu<'a>>,
}

impl<'a> TuiTextField<'a> {
    pub fn menu(mut self, menu: TuiMenu<'a>) -> Self {
        self._menu = Some(menu);
        self
    }

    pub fn prompt(mut self, prompt: &'a str) -> Self {
        self._prompt = prompt;
        self
    }

    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self._prefix = prefix;
        self
    }

    pub fn suffix(mut self, suffix: &'a str) -> Self {
        self._suffix = suffix;
        self
    }

    pub fn get_input(
        &mut self,
        backend: &mut TuiBackend,
        context: &JoshutoContext,
    ) -> Option<String> {
        context.events.flush();

        let mut line_buffer = line_buffer::LineBuffer::with_capacity(255);
        let completer = FilenameCompleter::new();

        let mut completion_tracker: Option<CompletionTracker> = None;

        let char_idx = self._prefix.chars().map(|c| c.len_utf8()).sum();

        line_buffer.insert_str(0, self._suffix);
        line_buffer.insert_str(0, self._prefix);
        line_buffer.set_pos(char_idx);

        let terminal = backend.terminal_mut();

        loop {
            terminal
                .draw(|mut frame| {
                    let f_size: Rect = frame.size();
                    if f_size.height == 0 {
                        return;
                    }

                    {
                        let mut view = TuiView::new(&context);
                        view.show_bottom_status = false;
                        frame.render_widget(view, f_size);
                    }

                    if let Some(menu) = self._menu.take() {
                        let menu_len = menu.len();
                        let menu_y = if menu_len + 2 > f_size.height as usize {
                            0
                        } else {
                            (f_size.height as usize - menu_len - 2) as u16
                        };

                        let rect = Rect {
                            x: 0,
                            y: menu_y,
                            width: f_size.width,
                            height: menu_len as u16,
                        };
                        frame.render_widget(menu, rect);
                    }

                    let cursor_xpos = line_buffer.pos();

                    let cmd_prompt_style = Style::default().fg(Color::LightGreen);
                    let cursor_style = Style::default().modifier(Modifier::REVERSED);

                    let prefix = &line_buffer.as_str()[..cursor_xpos];

                    let curr = line_buffer.as_str()[cursor_xpos..].chars().nth(0);
                    let (suffix, curr) = match curr {
                        Some(c) => {
                            let curr_len = c.len_utf8();
                            (&line_buffer.as_str()[(cursor_xpos + curr_len)..], c)
                        }
                        None => ("", ' '),
                    };

                    let curr_string = curr.to_string();

                    let text = [
                        Text::styled(self._prompt, cmd_prompt_style),
                        Text::raw(prefix),
                        Text::styled(curr_string, cursor_style),
                        Text::raw(suffix),
                    ];

                    let textfield_rect = Rect {
                        x: 0,
                        y: f_size.height - 1,
                        width: f_size.width,
                        height: 1,
                    };

                    frame.render_widget(Paragraph::new(text.iter()).wrap(true), textfield_rect);
                })
                .unwrap();

            if let Ok(event) = context.events.next() {
                match event {
                    Event::Input(key) => {
                        match key {
                            Key::Backspace => {
                                if line_buffer.backspace(1) {
                                    completion_tracker.take();
                                }
                            }
                            Key::Left => {
                                if line_buffer.move_backward(1) {
                                    completion_tracker.take();
                                }
                            }
                            Key::Right => {
                                if line_buffer.move_forward(1) {
                                    completion_tracker.take();
                                }
                            }
                            Key::Delete => {
                                if line_buffer.delete(1).is_some() {
                                    completion_tracker.take();
                                }
                            }
                            Key::Home => {
                                line_buffer.move_home();
                                completion_tracker.take();
                            }
                            Key::End => {
                                line_buffer.move_end();
                                completion_tracker.take();
                            }
                            Key::Up => {}
                            Key::Down => {}
                            Key::Esc => {
                                return None;
                            }
                            Key::Char('\t') => {
                                if completion_tracker.is_none() {
                                    let res = completer
                                        .complete_path(line_buffer.as_str(), line_buffer.pos());
                                    if let Ok((pos, mut candidates)) = res {
                                        candidates.sort_by(|x, y| {
                                            x.display()
                                                .partial_cmp(y.display())
                                                .unwrap_or(std::cmp::Ordering::Less)
                                        });
                                        let ct = CompletionTracker::new(
                                            pos,
                                            candidates,
                                            String::from(line_buffer.as_str()),
                                        );
                                        completion_tracker = Some(ct);
                                    }
                                }

                                if let Some(ref mut s) = completion_tracker {
                                    if s.index < s.candidates.len() {
                                        let candidate = &s.candidates[s.index];
                                        completer.update(
                                            &mut line_buffer,
                                            s.pos,
                                            candidate.display(),
                                        );
                                        s.index += 1;
                                    }
                                }
                            }
                            Key::Char('\n') => {
                                break;
                            }
                            Key::Char(c) => {
                                if line_buffer.insert(c, 1).is_some() {
                                    completion_tracker.take();
                                }
                            }
                            _ => {}
                        }
                        context.events.flush();
                    }
                    _ => {}
                };
            }
        }
        if line_buffer.as_str().is_empty() {
            None
        } else {
            let input_string = line_buffer.to_string();
            Some(input_string)
        }
    }
}

impl<'a> std::default::Default for TuiTextField<'a> {
    fn default() -> Self {
        Self {
            _prompt: "",
            _prefix: "",
            _suffix: "",
            _menu: None,
        }
    }
}

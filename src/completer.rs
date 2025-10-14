use crate::{ColorTheme, Syntax, Token, TokenType, format_token};
use egui::{
    Event, Frame, Modifiers, Sense, Stroke, TextBuffer, text::CCursor, text_edit::TextEditOutput,
    text_selection::text_cursor_state::ccursor_previous_word,
};
use trie::Trie;

mod trie {
    #![allow(dead_code)]
    use std::{iter::Peekable, str::Chars};

    const ROOT_CHAR: char = ' ';

    #[derive(Debug, Clone)]
    pub struct Trie {
        root: char,
        is_word: bool,
        leaves: Vec<Trie>,
    }

    impl PartialEq for Trie {
        fn eq(&self, other: &Self) -> bool {
            self.root == other.root
        }
    }
    impl PartialOrd for Trie {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Trie {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.root.cmp(&other.root)
        }
    }
    impl Eq for Trie {}

    impl Default for Trie {
        fn default() -> Self {
            Self {
                root: ROOT_CHAR,
                is_word: false,
                leaves: vec![],
            }
        }
    }

    impl Trie {
        pub fn new(root: char) -> Self {
            Trie {
                root,
                ..Default::default()
            }
        }
        pub fn clear(&mut self) {
            self.leaves.clear();
        }
        pub fn push(&mut self, word: &str) {
            self.push_chars(&mut word.chars());
        }

        pub fn push_chars(&mut self, word: &mut Chars) {
            if let Some(first) = word.next() {
                if let Some(leaf) = self.leaves.iter_mut().find(|l| l.root == first) {
                    leaf.push_chars(word)
                } else {
                    let mut new = Trie::new(first);
                    new.push_chars(word);
                    self.leaves.push(new);
                }
            } else {
                self.is_word = true;
                self.leaves.sort();
                self.leaves.reverse();
            }
        }

        pub fn from_words(words: &[&str]) -> Self {
            let mut trie = Trie::new(ROOT_CHAR);
            words.iter().for_each(|w| {
                trie.push_chars(&mut w.chars());
            });
            trie
        }

        pub fn words(&self) -> Vec<String> {
            let mut words = vec![];
            for child in self.leaves.iter() {
                child.words_recursive("", &mut words);
            }
            words.reverse();
            words
        }
        fn words_recursive(&self, prefix: &str, words: &mut Vec<String>) {
            let mut prefix = prefix.to_string();
            prefix.push(self.root);
            if self.is_word {
                words.push(prefix.clone());
            }
            for child in self.leaves.iter() {
                child.words_recursive(&prefix, words);
            }
        }

        pub fn find_completions(&self, prefix: &str) -> Vec<String> {
            self.find_by_prefix(prefix)
                .map(|t| t.words())
                .unwrap_or_default()
        }
        pub fn find_by_prefix(&self, prefix: &str) -> Option<&Trie> {
            let mut found = None;
            let mut start = " ".to_string();
            start.push_str(prefix);
            let mut part = start.chars().peekable();
            self.find_recursice(&mut part, &mut found);
            found
        }
        fn find_recursice<'a>(&'a self, part: &mut Peekable<Chars>, found: &mut Option<&'a Trie>) {
            if let Some(c) = part.next()
                && self.root == c
            {
                if part.peek().is_none() {
                    *found = Some(self);
                }
                self.leaves
                    .iter()
                    .for_each(|l| l.find_recursice(&mut part.clone(), found))
            }
        }
    }
}
pub fn trie_from_syntax(syntax: &Syntax) -> Trie {
    let mut trie = Trie::default();

    syntax.keywords.iter().for_each(|word| trie.push(word));
    syntax.types.iter().for_each(|word| trie.push(word));
    syntax.special.iter().for_each(|word| trie.push(word));
    if !syntax.case_sensitive {
        syntax
            .keywords
            .iter()
            .for_each(|word| trie.push(&word.to_lowercase()));
        syntax
            .types
            .iter()
            .for_each(|word| trie.push(&word.to_lowercase()));
        syntax
            .special
            .iter()
            .for_each(|word| trie.push(&word.to_lowercase()));
    }
    trie
}

#[derive(Default, Debug, Clone)]
pub struct Completer {
    prefix: String,
    cursor: CCursor,
    ignore_cursor: Option<usize>,
    trie_syntax: Trie,
    trie_user: Option<Trie>,
    variant_id: usize,
    completions: Vec<String>,
}

/// Completer shoud be stored somewhere in your App struct.
/// In future releases will be replaced with trait.
impl Completer {
    pub fn new_with_syntax(syntax: &Syntax) -> Self {
        Completer {
            trie_syntax: trie_from_syntax(syntax),
            ..Default::default()
        }
    }
    pub fn with_user_words(self) -> Self {
        Completer {
            trie_user: Some(Trie::default()),
            ..self
        }
    }

    /// If using Completer without CodeEditor this method should be called before text-editing widget.
    /// Up/Down arrows for selection, Tab for completion, Esc for hiding
    pub fn handle_input(&mut self, ctx: &egui::Context) {
        if self.prefix.is_empty() {
            return;
        }
        if let Some(cursor) = self.ignore_cursor
            && cursor == self.cursor.index
        {
            return;
        }

        let mut completions_syntax = self.trie_syntax.find_completions(&self.prefix);
        completions_syntax.reverse();
        let mut completions_user = self
            .trie_user
            .as_ref()
            .map(|t| t.find_completions(&self.prefix))
            .unwrap_or_default();
        completions_user.reverse();
        self.completions = [completions_syntax, completions_user].concat();
        if self.completions.is_empty() {
            return;
        }
        let last = self.completions.len().saturating_sub(1);
        ctx.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, egui::Key::Escape) {
                self.ignore_cursor = Some(self.cursor.index);
            } else if i.consume_key(Modifiers::NONE, egui::Key::ArrowDown) {
                self.variant_id = if self.variant_id == last {
                    0
                } else {
                    self.variant_id.saturating_add(1).min(last)
                };
            } else if i.consume_key(Modifiers::NONE, egui::Key::ArrowUp) {
                self.variant_id = if self.variant_id == 0 {
                    last
                } else {
                    self.variant_id.saturating_sub(1)
                };
            } else if i.consume_key(Modifiers::NONE, egui::Key::Tab) {
                let completion = self
                    .completions
                    .get(self.variant_id)
                    .map(String::from)
                    .unwrap_or_default();
                i.events.push(Event::Paste(completion));
            }
        });
    }

    /// If using Completer without CodeEditor this method should be called after text-editing widget as it uses &mut TextEditOutput
    pub fn show(
        &mut self,
        syntax: &Syntax,
        theme: &ColorTheme,
        fontsize: f32,
        editor_output: &mut TextEditOutput,
    ) {
        let ctx = editor_output.response.ctx.clone();
        let galley = &editor_output.galley;

        if editor_output.response.changed() {
            // Update Competer Dictionary
            if let Some(trie_user) = self.trie_user.as_mut() {
                trie_user.clear();
                Token::default()
                    .tokens(syntax, galley.text())
                    .iter()
                    .filter(|t| matches!(t.ty(), TokenType::Literal | TokenType::Function))
                    .for_each(|t| trie_user.push(t.buffer()));
            }
        }

        // Auto-Completer
        let cursor_range = editor_output.state.cursor.char_range();
        if let Some(range) = cursor_range {
            let cursor = range.primary;
            let cursor_pos_in_galley = galley.pos_from_cursor(cursor);
            let cursor_rect =
                cursor_pos_in_galley.translate(editor_output.response.rect.left_top().to_vec2());
            // let cursor_on_screen = editor_output.response.rect.left_top()
            // + cursor_pos_in_galley.left_bottom().to_vec2();
            let word_start = ccursor_previous_word(galley.text(), cursor);
            if self.cursor != cursor {
                self.cursor = cursor;
                self.prefix.clear();
                // self.completions.clear();
                self.ignore_cursor = None;
                self.variant_id = 0;
            }

            if self.ignore_cursor.is_some_and(|c| c == self.cursor.index) {
                editor_output.response.request_focus();
                return;
            } else {
                self.ignore_cursor = None;
            }
            let next_char_allows = galley
                .chars()
                .nth(cursor.index)
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
                || (range.secondary.index > range.primary.index);

            self.prefix = if next_char_allows {
                let prefix = galley
                    .text()
                    .char_range(word_start.index..cursor.index)
                    .to_string();
                if let Some((_, tail)) =
                    prefix.rsplit_once(|c: char| !(c.is_alphanumeric() || c == '_'))
                {
                    tail.to_string()
                } else {
                    prefix
                }
            } else {
                String::new()
            };
            if !(self.prefix.is_empty() || self.completions.is_empty()) {
                egui::Popup::new(
                    egui::Id::new("Completer"),
                    ctx.clone(),
                    cursor_rect,
                    editor_output.response.layer_id,
                )
                .frame(Frame::popup(&ctx.style()).fill(theme.bg()))
                .sense(Sense::empty())
                .show(|ui| {
                    ui.response().sense = Sense::empty();
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                    for (i, completion) in self.completions.iter().enumerate() {
                        let word = format!("{}{completion}", &self.prefix);
                        let token_type = match &word {
                            word if syntax.is_keyword(word) => TokenType::Keyword,
                            word if syntax.is_special(word) => TokenType::Special,
                            word if syntax.is_type(word) => TokenType::Type,
                            _ => TokenType::Literal,
                        };
                        let fmt = format_token(theme, fontsize, token_type);
                        let colored_text = egui::text::LayoutJob::single_section(word, fmt);
                        let selected = i == self.variant_id;
                        ui.add(
                            egui::Button::new(colored_text)
                                .sense(Sense::empty())
                                .frame(true)
                                .fill(theme.bg())
                                .stroke(if selected {
                                    Stroke::new(
                                        ui.style().visuals.widgets.hovered.bg_stroke.width,
                                        theme.type_color(TokenType::Literal),
                                    )
                                } else {
                                    Stroke::NONE
                                }),
                        );
                    }
                });
            }
        }
    }
}

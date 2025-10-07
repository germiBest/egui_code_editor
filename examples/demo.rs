#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{self, CreationContext, egui};
use egui_code_editor::{
    self, CodeEditor, ColorTheme, Editor, Syntax, TokenType, highlighting::Token,
};

const THEMES: [ColorTheme; 8] = [
    ColorTheme::AYU,
    ColorTheme::AYU_MIRAGE,
    ColorTheme::AYU_DARK,
    ColorTheme::GITHUB_DARK,
    ColorTheme::GITHUB_LIGHT,
    ColorTheme::GRUVBOX,
    ColorTheme::GRUVBOX_LIGHT,
    ColorTheme::SONOKAI,
];

const SYNTAXES: [SyntaxDemo; 5] = [
    SyntaxDemo::new(
        "Lua",
        r#"-- Binary Search
function binarySearch(list, value)
    local function search(low, high)
        if low > high then return false end
        local mid = math.floor((low+high)/2)
        if list[mid] > value then return search(low,mid-1) end
        if list[mid] < value then return search(mid+1,high) end
        return mid
    end
    return search(1,#list)
end"#,
    ),
    SyntaxDemo::new(
        "Python",
        r#"from collections.abc import Iterable
from typing import Protocol

class Combiner(Protocol):
    def __call__(self, *vals: bytes, maxlen: int | None = None) -> list[bytes]: ...

def batch_proc(data: Iterable[bytes], cb_results: Combiner) -> bytes:
    for item in data:
        ...

def good_cb(*vals: bytes, maxlen: int | None = None) -> list[bytes]:
    ...
"""
def bad_cb(*vals: bytes, maxitems: int | None) -> list[bytes]:
    ...
"""
batch_proc([], good_cb)  # OK
batch_proc([], bad_cb)   # Error! Argument 2 has incompatible type because of
                         # different name and kind in the callback"#,
    ),
    SyntaxDemo::new(
        "Rust",
        r#"// Code Editor
CodeEditor::default()
    .id_source("code editor")
    .with_rows(12)
    .with_fontsize(14.0)
    .with_theme(self.theme)
    .with_syntax(self.syntax.to_owned())
    .with_numlines(true)
    .vscroll(true)
    .show(ui, &mut self.code);"#,
    ),
    SyntaxDemo::new(
        "Shell",
        r#"#!/bin/bash
user=p4ymak
if grep $user /etc/passwd
then
echo "The user $user Exists"
fi"#,
    ),
    SyntaxDemo::new(
        "SQL",
        r#"select now(); -- what time it is?
WITH employee_ranking AS (
  SELECT 
    employee_id as real, 
    last_name, 
    first_name, 
    salary, 
    dept_id
    RANK() OVER (PARTITION BY dept_id ORDER BY salary DESC) as ranking
  FROM employee
)"#,
    ),
];

#[derive(Clone, Copy)]
struct SyntaxDemo {
    name: &'static str,
    example: &'static str,
}

impl SyntaxDemo {
    const fn new(name: &'static str, example: &'static str) -> Self {
        SyntaxDemo { name, example }
    }
    fn syntax(&self) -> Syntax {
        match self.name {
            "Assembly" => Syntax::asm(),
            "Lua" => Syntax::lua(),
            "Python" => Syntax::python(),
            "Rust" => Syntax::rust(),
            "Shell" => Syntax::shell(),
            "SQL" => Syntax::sql(),
            _ => Syntax::shell(),
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    #[cfg(debug_assertions)]
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(true)
            .with_transparent(false)
            .with_resizable(true)
            .with_maximized(false)
            .with_drag_and_drop(true)
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([280.0, 280.0]),

        ..Default::default()
    };

    eframe::run_native(
        "Egui Code Editor Demo",
        options,
        Box::new(|cc| Ok(Box::new(CodeEditorDemo::new(cc)))),
    )
}

#[derive(Default)]
struct CodeEditorDemo {
    code: String,
    theme: ColorTheme,
    syntax: Syntax,
    example: bool,
    shift: isize,
    numlines_only_natural: bool,
}
impl CodeEditorDemo {
    fn new(_cc: &CreationContext) -> Self {
        let rust = SYNTAXES[2];
        CodeEditorDemo {
            code: rust.example.to_string(),
            theme: ColorTheme::GRUVBOX,
            syntax: rust.syntax(),
            example: true,
            shift: 0,
            numlines_only_natural: false,
        }
    }
}
impl eframe::App for CodeEditorDemo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("theme_picker").show(ctx, |ui| {
            ui.heading("Theme");
            egui::ScrollArea::both().show(ui, |ui| {
                for theme in THEMES.iter() {
                    if ui
                        .selectable_value(&mut self.theme, *theme, theme.name())
                        .clicked()
                    {
                        if theme.is_dark() {
                            ctx.set_visuals(egui::Visuals::dark());
                        } else {
                            ctx.set_visuals(egui::Visuals::light());
                        }
                    };
                }
            });
        });

        egui::SidePanel::right("syntax_picker").show(ctx, |ui| {
            ui.horizontal(|h| {
                h.heading("Syntax");
                h.checkbox(&mut self.example, "Example");
            });
            egui::ScrollArea::both().show(ui, |ui| {
                for syntax in SYNTAXES.iter() {
                    if ui
                        .selectable_label(self.syntax.language() == syntax.name, syntax.name)
                        .clicked()
                    {
                        self.syntax = syntax.syntax();
                        if self.example {
                            self.code = syntax.example.to_string()
                        }
                    };
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|h| {
                h.label("Numbering Shift");
                h.add(egui::DragValue::new(&mut self.shift));
                h.checkbox(&mut self.numlines_only_natural, "Only Natural Numbering");
            });
            let mut editor = CodeEditor::default()
                .id_source("code editor")
                .with_rows(10)
                .with_fontsize(14.0)
                .with_theme(self.theme)
                .with_syntax(self.syntax.to_owned())
                .with_numlines(true)
                .with_numlines_shift(self.shift)
                .with_numlines_only_natural(self.numlines_only_natural)
                .vscroll(true);
            let output = editor.show(ui, &mut self.code);
            let galley = output.galley;
            // Get the current cursor state and range

            let cursor_state = output.state.cursor;
            let cursor_range = cursor_state.char_range();
            if let Some(range) = cursor_range {
                // Convert the primary cursor's char index to a position within the galley
                let cursor_pos_in_galley = galley.pos_from_cursor(range.primary);
                // Calculate the absolute on-screen position by adding the widget's screen position
                let cursor_on_screen = output.response.rect.left_top()
                    + cursor_pos_in_galley.left_top().to_vec2()
                    + egui::Vec2::new(0.0, 20.0);
                // `cursor_on_screen` now holds the cursor's position on the screen

                let word = galley
                    .text()
                    .get(0..range.primary.index)
                    .and_then(|r| r.rsplit_once(' '))
                    .map(|s| s.1.trim());
                if let Some(prefix) = word
                    && !prefix.is_empty()
                {
                    let completions = editor.find_completions(prefix);
                    if !completions.is_empty() {
                        egui::Window::new("completions")
                            .title_bar(false)
                            .resizable(false)
                            .current_pos(cursor_on_screen)
                            .show(ui.ctx(), |ui| {
                                // if let Some(prefix) = self.code.split_whitespace().last() {
                                for completion in completions.iter() {
                                    let syntax = editor.syntax();
                                    let word = format!(
                                        "{}{completion}",
                                        if self.syntax.case_sensitive {
                                            prefix
                                        } else {
                                            &prefix.to_uppercase()
                                        }
                                    );
                                    let token_type = match &word {
                                        word if syntax.is_keyword(word) => TokenType::Keyword,
                                        word if syntax.is_special(word) => TokenType::Special,
                                        word if syntax.is_type(word) => TokenType::Type,
                                        _ => TokenType::Literal,
                                    };
                                    let fmt = editor.format(token_type);
                                    let colored = egui::text::LayoutJob::single_section(word, fmt);
                                    ui.button(colored);
                                }
                            });
                    }
                }
            }

            ui.separator();

            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for token in Token::default().tokens(&self.syntax, &self.code) {
                        ui.horizontal(|h| {
                            let fmt = editor.format(token.ty());
                            h.label(egui::text::LayoutJob::single_section(
                                format!("{:?}", token.ty()),
                                fmt,
                            ));
                            h.label(token.buffer());
                        });
                    }
                });
        });
    }
}

use owo_colors::OwoColorize;
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};
use terminal_size::*;

use crate::data::modinfo::ModInfoFull;

pub fn print_in_grid(items: Vec<impl ToString>, column_hint: usize) {
    let width = if let Some((Width(w), Height(_h))) = terminal_size() {
        w - 2
    } else {
        72
    };

    let mut grid = Grid::new(GridOptions {
        filling: Filling::Spaces(2),
        direction: Direction::LeftToRight,
    });
    for item in items {
        grid.add(Cell::from(item.to_string()));
    }

    if let Some(g) = grid.fit_into_width(width.into()) {
        // https://github.com/ogham/rust-term-grid/issues/11
        println!("{}", g);
    } else {
        println!("{}", grid.fit_into_columns(column_hint));
    }
}

/// Given a count, return a string with the count + the word `mod` pluralized for English.
pub fn pluralize_mod(count: usize) -> String {
    if count == 1 {
        format!("{} mod", "one".blue())
    } else {
        format!("{} mods", count.blue())
    }
}

pub fn emit_modlist_with_caption(modlist: Vec<ModInfoFull>, caption: &str) {
    if !modlist.is_empty() {
        println!(
            "{} {}:",
            pluralize_mod(modlist.len()).bold(),
            caption.bold()
        );
        print_in_grid(modlist.iter().map(|xs| xs.mod_id()).collect(), 10);
    }
}

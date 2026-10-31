//! StarDork, Ryan Kulla's ncurses game (stardork.c 0.7, 2011, GPLv2),
//! ported to Rust and crossterm for the Asciiverse catalog. Same screen,
//! keys, rules and timings; what changed is listed in PATCHES.md.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute},
    terminal,
};

const YMIN: i32 = 1;
const XMIN: i32 = 0;
const START_POS_Y: i32 = 18;
const START_POS_X: i32 = 45;
const LEVEL_MAX: i32 = 30;
const HUD_START_POS: i32 = 17;
const PROBE_SPEED_MS: u64 = 5;

const SHIP: char = 'X';
const STAR: char = '.';
const WORMHOLE: char = '@';
const PROBE: char = '+';
const BLANK: char = ' ';

/// The smallest terminal the fixed layout fits: the ship starts at row 18,
/// column 45, and the status line runs to column 61.
const MIN_COLS: u16 = 64;
const MIN_ROWS: u16 = 20;

/// What a key does.
enum Key {
    Move(i32, i32),
    Shoot(i32, i32),
    Quit,
    Other,
}

fn key_of(k: KeyEvent) -> Key {
    if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c' | 'C')) {
        return Key::Quit;
    }
    match k.code {
        KeyCode::Up | KeyCode::Char('8') => Key::Move(-1, 0),
        KeyCode::Down | KeyCode::Char('2') => Key::Move(1, 0),
        KeyCode::Right | KeyCode::Char('6') => Key::Move(0, 1),
        KeyCode::Left | KeyCode::Char('4') => Key::Move(0, -1),
        KeyCode::Home | KeyCode::Char('7') => Key::Move(-1, -1),
        KeyCode::PageUp | KeyCode::Char('9') => Key::Move(-1, 1),
        KeyCode::End | KeyCode::Char('1') => Key::Move(1, -1),
        KeyCode::PageDown | KeyCode::Char('3') => Key::Move(1, 1),
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            // the probe launcher: diagonals move two columns per row
            'd' => Key::Shoot(1, 0),
            'a' => Key::Shoot(0, -1),
            's' => Key::Shoot(-1, 0),
            'f' => Key::Shoot(0, 1),
            'z' => Key::Shoot(-1, -2),
            'x' => Key::Shoot(-1, 2),
            'c' => Key::Shoot(1, -2),
            'v' => Key::Shoot(1, 2),
            'q' => Key::Quit,
            _ => Key::Other,
        },
        KeyCode::Esc => Key::Quit,
        _ => Key::Other,
    }
}

/// The terminal, drawn like curses: cells are queued and sent at
/// `refresh`. A copy of the screen lets a colour change repaint it all,
/// as curses does when the colour pair is redefined.
struct Screen {
    out: io::Stdout,
    lines: i32,
    cols: i32,
    cells: Vec<char>,
    color: Color,
    bold: bool,
}

impl Screen {
    fn put(&mut self, y: i32, x: i32, c: char) {
        if y < 0 || x < 0 || y >= self.lines || x >= self.cols {
            return; // curses ignores what falls off the screen
        }
        // like curses, send only what changes
        let cell = &mut self.cells[(y * self.cols + x) as usize];
        if *cell != c {
            *cell = c;
            let _ = queue!(self.out, cursor::MoveTo(x as u16, y as u16), Print(c));
        }
    }

    fn text(&mut self, y: i32, x: i32, s: &str) {
        for (i, c) in s.chars().enumerate() {
            self.put(y, x + i as i32, c);
        }
    }

    fn refresh(&mut self) {
        let _ = self.out.flush();
    }

    fn nap(&mut self, ms: u64) {
        self.refresh();
        std::thread::sleep(Duration::from_millis(ms));
    }

    /// The colour of everything on screen, as `change_colors` sets it.
    fn set_color(&mut self, color: Color) {
        if color == self.color {
            return;
        }
        self.color = color;
        // the plain SGR codes curses sends for COLOR_RED and COLOR_WHITE
        let sgr = if color == Color::DarkRed { "\x1b[31m" } else { "\x1b[37m" };
        let _ = queue!(self.out, Print(sgr));
        for y in 0..self.lines {
            let row: String = self.cells[(y * self.cols) as usize..((y + 1) * self.cols) as usize].iter().collect();
            let _ = queue!(self.out, cursor::MoveTo(0, y as u16), Print(row));
        }
    }

    fn set_bold(&mut self, bold: bool) {
        self.bold = bold;
        let _ = queue!(self.out, SetAttribute(if bold { Attribute::Bold } else { Attribute::NormalIntensity }));
    }

    fn clear(&mut self) {
        self.cells.fill(BLANK);
        let _ = queue!(self.out, terminal::Clear(terminal::ClearType::All));
    }
}

/// Waits for a key press. Ctrl+C, q or Esc says goodbye from anywhere, as
/// the original's SIGINT handler did.
fn getch(scr: &mut Screen) -> Key {
    scr.refresh();
    loop {
        match event::read() {
            Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => {
                let key = key_of(k);
                if let Key::Quit = key {
                    bye(scr);
                }
                return key;
            }
            Ok(_) => {}
            Err(_) => bye(scr),
        }
    }
}

fn restore(scr: &mut Screen) {
    let _ = queue!(scr.out, SetAttribute(Attribute::Reset), ResetColor, cursor::Show, terminal::LeaveAlternateScreen);
    let _ = scr.out.flush();
    let _ = terminal::disable_raw_mode();
}

fn bye(scr: &mut Screen) -> ! {
    scr.set_bold(false);
    scr.text(5, 10, "Bye Bye!");
    scr.nap(1000);
    restore(scr);
    std::process::exit(0);
}

struct Game {
    scr: Screen,
    rng: fastrand::Rng,
    power: i32,
    level: i32,
    lives: i32,
    moves: i32,
    /// Every star ever plotted, one batch per level, as (row, column).
    stars: Vec<(i32, i32)>,
    wormhole: (i32, i32),
}

impl Game {
    fn out_up(&self, y: i32) -> bool {
        y == YMIN
    }
    fn out_down(&self, y: i32) -> bool {
        y == self.scr.lines - 1
    }
    fn out_left(&self, x: i32) -> bool {
        x == XMIN
    }
    fn out_right(&self, x: i32) -> bool {
        x == self.scr.cols - 1
    }

    fn display_hud(&mut self) {
        for i in 1..=9 {
            let c = if i <= self.power { '|' } else { BLANK };
            self.scr.put(0, i + HUD_START_POS, c);
        }
        self.scr.put(0, 10 + HUD_START_POS, BLANK);
        self.scr.text(0, 11, "Power: ");
        self.scr.text(0, 30, &format!("Lives: {}", self.lives));
        self.scr.text(0, 40, &format!("Level: {}", self.level));
    }

    fn count_moves(&mut self) {
        self.moves += 1;
        self.scr.text(0, 50, &format!("Moves: {}", self.moves));
    }

    fn display_wormhole(&mut self) {
        self.scr.put(self.wormhole.0, self.wormhole.1, WORMHOLE);
    }

    /// One more batch of stars: one per column but the first, on random
    /// rows below the status line (the last row of a batch falls just off
    /// the screen, as in the original).
    fn plot_stars_randomly(&mut self) {
        for _ in 1..self.scr.cols {
            let r1 = self.rng.i32(0..self.scr.lines) + 1;
            let r2 = self.rng.i32(0..self.scr.cols);
            self.stars.push((r1, r2));
        }
    }

    fn show_stars(&mut self) {
        for i in 0..self.stars.len() {
            let (y, x) = self.stars[i];
            self.scr.put(y, x, STAR);
        }
    }

    /// A new wormhole, somewhere on the playfield with no star on it.
    fn plot_wormhole(&mut self) {
        loop {
            let r1 = self.rng.i32(YMIN..self.scr.lines);
            let r2 = self.rng.i32(0..self.scr.cols);
            if !self.stars.contains(&(r1, r2)) {
                self.wormhole = (r1, r2);
                return;
            }
        }
    }

    fn check_if_wormhole_is_hit(&mut self, y: i32, x: i32) -> bool {
        if (y, x) != self.wormhole {
            return false;
        }
        self.plot_wormhole();
        self.level += 1;
        self.plot_stars_randomly();
        if self.level == LEVEL_MAX {
            self.winner();
        }
        true
    }

    /// Each star under the ship takes one unit of power; the screen turns
    /// red until a key is pressed, once per star.
    fn check_if_bumped(&mut self, y: i32, x: i32) {
        let hits = self.stars.iter().filter(|&&s| s == (y, x)).count();
        for _ in 0..hits {
            self.scr.put(y, x, SHIP);
            self.scr.set_color(Color::DarkRed);
            getch(&mut self.scr);
            self.power -= 1;
        }
        self.scr.set_color(Color::Grey);
        self.check_if_dead(y, x);
    }

    fn check_if_dead(&mut self, y: i32, x: i32) {
        if self.power < 1 {
            self.lives -= 1;
            self.power = 9;
            if self.lives < 1 {
                self.game_over(y, x);
            }
        }
        self.show_stars();
    }

    fn move_ship(&mut self, y: &mut i32, x: &mut i32, dy: i32, dx: i32) {
        let blocked = (dy < 0 && self.out_up(*y))
            || (dy > 0 && self.out_down(*y))
            || (dx < 0 && self.out_left(*x))
            || (dx > 0 && self.out_right(*x));
        if !blocked {
            self.scr.put(*y, *x, BLANK);
            *y += dy;
            *x += dx;
            self.scr.put(*y, *x, SHIP);
        }
    }

    /// A probe flies from the ship until it leaves the screen or reaches
    /// the wormhole, drawn and wiped one cell at a time.
    fn shoot(&mut self, y: i32, x: i32, dy: i32, dx: i32) {
        let (lines, cols) = (self.scr.lines, self.scr.cols);
        let (mut py, mut px) = (y, x);
        // the original's loop bounds, direction by direction
        let mut steps_x = x;
        loop {
            let more = match (dy, dx) {
                (0, 1) => px < cols - 1,
                (-1, 0) => py > 1,
                (1, 0) => py < lines,
                (0, -1) => px > 0,
                (-1, -2) => py > 1 && steps_x > 1,
                (-1, 2) => py > 1 && steps_x < cols - 1,
                (1, -2) => py < lines && steps_x > 1,
                (1, 2) => py < lines && steps_x < cols - 1,
                _ => false,
            };
            if !more {
                break;
            }
            py += dy;
            px += dx;
            steps_x += dx.signum();
            self.scr.put(py, px, PROBE);
            self.scr.nap(PROBE_SPEED_MS);
            self.scr.put(py, px, BLANK);
            self.show_stars();
            if self.check_if_wormhole_is_hit(py, px) {
                break;
            }
        }
    }

    fn game_over(&mut self, y: i32, x: i32) -> ! {
        let (lines, cols) = (self.scr.lines, self.scr.cols);
        self.scr.set_bold(true);
        self.scr.put(y, x, SHIP);
        getch(&mut self.scr);
        self.scr.put(y, x, BLANK);

        let (mut i, mut j) = (1, lines);
        while i < lines / 2 - 2 {
            i += 1;
            j -= 1;
            for (k, (top, bottom)) in [('G', 'O'), ('A', 'V'), ('M', 'E'), ('E', 'R')].into_iter().enumerate() {
                let col = cols / 2 - 30 + 10 * k as i32;
                self.scr.put(i, col, BLANK);
                self.scr.put(i + 1, col, top);
                self.scr.put(j, col, BLANK);
                self.scr.put(j - 1, col, bottom);
            }
            self.scr.nap(100);
        }
        getch(&mut self.scr);
        restore(&mut self.scr);
        std::process::exit(0);
    }

    fn winner(&mut self) -> ! {
        let cols = self.scr.cols;
        self.scr.clear();
        self.scr.set_bold(true);
        let name = ["LOGNAME", "USER", "USERNAME"].iter().find_map(|v| std::env::var(v).ok()).unwrap_or_else(|| " ".into());
        let won = format!("You win {name}, in {} moves", self.moves);
        self.scr.text(4, cols / 2 - won.chars().count() as i32 / 2, &won);
        for (i, c) in "You are a true stardork.".chars().enumerate() {
            self.scr.put(12, i as i32 + cols / 2 - 12, c);
            self.scr.nap(200);
        }
        self.scr.nap(200);
        getch(&mut self.scr);
        restore(&mut self.scr);
        std::process::exit(0);
    }

    fn control_ship(&mut self, mut y: i32, mut x: i32) {
        self.scr.set_color(Color::Grey);
        self.show_stars();
        loop {
            self.display_hud();
            self.display_wormhole();
            self.scr.put(y, x, SHIP);
            let key = getch(&mut self.scr);
            if let Key::Move(dy, dx) = key {
                self.move_ship(&mut y, &mut x, dy, dx);
            }
            self.count_moves();
            if let Key::Shoot(dy, dx) = key {
                self.shoot(y, x, dy, dx);
            }
            self.check_if_bumped(y, x);
            self.check_if_wormhole_is_hit(y, x);
        }
    }
}

fn usage() -> ! {
    println!(
        "usage: stardork [--seed N]\n\nStarDork 0.7: probe the wormholes (@) and keep clear of the stars (.).\n\
         Move: arrow keys, Home, PgUp, End, PgDn, or 1-9 on the number keys.\n\
         Shoot: a left, s up, d down, f right, z x c v diagonals. Quit: q or Ctrl+C."
    );
    std::process::exit(0);
}

fn main() {
    let mut rng = fastrand::Rng::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seed" => match args.next().and_then(|s| s.parse().ok()) {
                Some(seed) => rng.seed(seed),
                None => usage(),
            },
            "--version" | "-V" => {
                println!("stardork 0.7");
                return;
            }
            _ => usage(),
        }
    }

    let (cols, lines) = terminal::size().unwrap_or((80, 24));
    if cols < MIN_COLS || lines < MIN_ROWS {
        eprintln!("stardork needs a terminal of at least {MIN_COLS}x{MIN_ROWS}; this one is {cols}x{lines}.");
        std::process::exit(1);
    }
    let (cols, lines) = (cols as i32, lines as i32);

    let mut out = io::stdout();
    let _ = terminal::enable_raw_mode();
    let _ = queue!(out, terminal::EnterAlternateScreen, cursor::Hide, terminal::Clear(terminal::ClearType::All));
    let scr = Screen { out, lines, cols, cells: vec![BLANK; (lines * cols) as usize], color: Color::Reset, bold: false };
    let mut game = Game { scr, rng, power: 9, level: 1, lives: 3, moves: 0, stars: Vec::new(), wormhole: (0, 0) };

    game.plot_stars_randomly();
    game.plot_wormhole();
    game.scr.put(START_POS_Y, START_POS_X, SHIP);
    game.control_ship(START_POS_Y, START_POS_X);
}

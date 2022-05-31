#![deny(clippy::all, clippy::pedantic)]

// TODO: More styles

use std::collections::VecDeque;
use std::fmt::Display;
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossterm::cursor::MoveTo;
use crossterm::event::{read, Event, KeyCode};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};

use rand::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, Copy)]
enum SegmentType {
    NorthSouth,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
    EastWest,
}

impl SegmentType {
    fn from_next(a: Direction, b: Direction) -> SegmentType {
        match a {
            Direction::North => match b {
                Direction::North => SegmentType::NorthSouth,
                Direction::South => panic!(),
                Direction::East => SegmentType::SouthEast,
                Direction::West => SegmentType::SouthWest,
            },
            Direction::South => match b {
                Direction::North => panic!(),
                Direction::South => SegmentType::NorthSouth,
                Direction::East => SegmentType::NorthEast,
                Direction::West => SegmentType::NorthWest,
            },
            Direction::East => match b {
                Direction::North => SegmentType::NorthWest,
                Direction::South => SegmentType::SouthWest,
                Direction::East => SegmentType::EastWest,
                Direction::West => panic!(),
            },
            Direction::West => match b {
                Direction::North => SegmentType::NorthEast,
                Direction::South => SegmentType::SouthEast,
                Direction::East => panic!(),
                Direction::West => SegmentType::EastWest,
            },
        }
    }

    fn from_dir(a: Direction) -> SegmentType {
        match a {
            Direction::North | Direction::South => SegmentType::NorthSouth,
            Direction::East | Direction::West => SegmentType::EastWest,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Style {
    CurvedLine,
    SharpLine,
    Block,
}

impl Style {
    fn next(self) -> Style {
        match self {
            Style::CurvedLine => Style::SharpLine,
            Style::SharpLine => Style::Block,
            Style::Block => Style::CurvedLine,
        }
    }

    fn prev(self) -> Style {
        match self {
            Style::CurvedLine => Style::Block,
            Style::SharpLine => Style::CurvedLine,
            Style::Block => Style::SharpLine,
        }
    }
}

impl SegmentType {
    fn display(self, style: Style) -> char {
        match style {
            Style::CurvedLine => match self {
                SegmentType::NorthSouth => '│',
                SegmentType::NorthEast => '╰',
                SegmentType::NorthWest => '╯',
                SegmentType::SouthEast => '╭',
                SegmentType::SouthWest => '╮',
                SegmentType::EastWest => '─',
            },
            Style::SharpLine => match self {
                SegmentType::NorthSouth => '│',
                SegmentType::NorthEast => '└',
                SegmentType::NorthWest => '┘',
                SegmentType::SouthEast => '┌',
                SegmentType::SouthWest => '┐',
                SegmentType::EastWest => '─',
            },
            Style::Block => match self {
                SegmentType::NorthSouth => '█',
                SegmentType::NorthEast => '█',
                SegmentType::NorthWest => '█',
                SegmentType::SouthEast => '█',
                SegmentType::SouthWest => '█',
                SegmentType::EastWest => '█',
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Segment(u16, u16, SegmentType, Direction);

#[derive(Debug, Clone)]
struct GameState {
    snake: VecDeque<Segment>,
    delay: Duration,
    apple: (u16, u16),
    head: (u16, u16),
    board: (u16, u16),
    direction: Direction,
    style: Style,
}

impl GameState {
    fn new() -> Self {
        GameState {
            snake: {
                let mut v = VecDeque::new();
                v.push_back(Segment(0, 0, SegmentType::EastWest, Direction::East));
                v.push_back(Segment(1, 0, SegmentType::EastWest, Direction::East));
                v
            },
            delay: Duration::from_millis(250),
            apple: (5, 5),
            head: (1, 0),
            board: (10, 10),
            direction: Direction::East,
            style: Style::CurvedLine,
        }
    }
}

fn render_all(game: &GameState) {
    execute!(stdout(), Clear(ClearType::All)).unwrap();

    // Apple
    execute!(stdout(), MoveTo(game.apple.0, game.apple.1)).unwrap();
    print!("O");

    // Snake
    for Segment(x, y, s, _) in &game.snake {
        execute!(stdout(), MoveTo(*x, *y)).unwrap();
        print!("{}", s.display(game.style));
    }

    // Board
    execute!(stdout(), MoveTo(0, game.board.1)).unwrap();
    print!("{}", "─".repeat(game.board.0 as usize));
    for i in 0..game.board.1 {
        execute!(stdout(), MoveTo(game.board.0, i)).unwrap();
        print!("│");
    }
    execute!(stdout(), MoveTo(game.board.0, game.board.1)).unwrap();
    print!("╯");

    stdout().lock().flush().unwrap();
}

fn game_over() {
    println!("\nGame Over");
}

fn main() {
    enable_raw_mode().unwrap();
    execute!(stdout(), Hide).unwrap();

    let game = Arc::new(Mutex::new(GameState::new()));

    {
        let game = Arc::clone(&game);

        thread::spawn(move || loop {
            match read().unwrap() {
                Event::Key(k) => match k.code {
                    KeyCode::Char('q') => {
                        execute!(stdout(), Show).unwrap();
                        disable_raw_mode().unwrap();
                        println!();
                        std::process::exit(0);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let mut game = game.lock().unwrap();
                        if game.snake[game.snake.len() - 1].3 != Direction::South {
                            game.direction = Direction::North;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let mut game = game.lock().unwrap();
                        if game.snake[game.snake.len() - 1].3 != Direction::North {
                            game.direction = Direction::South;
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        let mut game = game.lock().unwrap();
                        if game.snake[game.snake.len() - 1].3 != Direction::East {
                            game.direction = Direction::West;
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        let mut game = game.lock().unwrap();
                        if game.snake[game.snake.len() - 1].3 != Direction::West {
                            game.direction = Direction::East;
                        }
                    }
                    KeyCode::Char('1') => {
                        let mut game = game.lock().unwrap();
                        game.board.0 = game.board.0.checked_sub(1).unwrap();
                        render_all(&game);
                    }
                    KeyCode::Char('2') => {
                        let mut game = game.lock().unwrap();
                        game.board.0 = game.board.0.checked_add(1).unwrap();
                        render_all(&game);
                    }
                    KeyCode::Char('3') => {
                        let mut game = game.lock().unwrap();
                        game.board.1 = game.board.1.checked_sub(1).unwrap();
                        render_all(&game);
                    }
                    KeyCode::Char('4') => {
                        let mut game = game.lock().unwrap();
                        game.board.1 = game.board.1.checked_add(1).unwrap();
                        render_all(&game);
                    }
                    KeyCode::Char('5') => {
                        let mut game = game.lock().unwrap();
                        game.delay = game.delay.checked_add(Duration::from_millis(20)).unwrap();
                    }
                    KeyCode::Char('6') => {
                        let mut game = game.lock().unwrap();
                        game.delay = game.delay.checked_sub(Duration::from_millis(20)).unwrap();
                    }
                    KeyCode::Char('7') => {
                        let mut game = game.lock().unwrap();
                        game.style = game.style.prev();
                        render_all(&game);
                    }
                    KeyCode::Char('8') => {
                        let mut game = game.lock().unwrap();
                        game.style = game.style.next();
                        render_all(&game);
                    }
                    _ => {}
                },
                _ => {}
            }
        });
    };

    let mut rng = thread_rng();

    loop {
        let head = game.lock().unwrap().head;
        let board = game.lock().unwrap().board;
        let direction = game.lock().unwrap().direction;
        let new_head = match direction {
            Direction::North => {
                if head.1 > 0 {
                    (head.0, head.1 - 1)
                } else {
                    break;
                }
            }
            Direction::South => {
                if head.1 + 1 < board.1 {
                    (head.0, head.1 + 1)
                } else {
                    break;
                }
            }
            Direction::West => {
                if head.0 > 0 {
                    (head.0 - 1, head.1)
                } else {
                    break;
                }
            }
            Direction::East => {
                if head.0 + 1 < board.0 {
                    (head.0 + 1, head.1)
                } else {
                    break;
                }
            }
        };
        {
            let mut game = game.lock().unwrap();
            if let Some(_) = game.snake.iter().find(|x| (x.0, x.1) == new_head) {
                break;
            }
            game.head = new_head;
            let len = game.snake.len();
            game.snake[len - 1].2 =
                SegmentType::from_next(game.snake[game.snake.len() - 1].3, game.direction);
            let segment = Segment(
                new_head.0,
                new_head.1,
                SegmentType::from_dir(game.direction),
                game.direction,
            );
            if new_head == game.apple {
                game.apple = (
                    rng.gen_range(0..game.board.0),
                    rng.gen_range(0..game.board.1),
                );
            } else {
                game.snake.pop_front();
            }
            game.snake.push_back(segment);
            render_all(&game);
        }
        let delay = game.lock().unwrap().delay;
        thread::sleep(delay);
    }

    game_over();

    execute!(stdout(), Show).unwrap();
    disable_raw_mode().unwrap();
    println!();
}

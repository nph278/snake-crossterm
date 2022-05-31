#![deny(clippy::all, clippy::pedantic)]

use std::collections::VecDeque;
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, Event, KeyCode};
use crossterm::execute;
use crossterm::style::{style, Color, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};

use rand::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    North,
    South,
    East,
    West,
}

// A type of segment in the snake, for printing
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
enum SnakeStyle {
    CurvedLine,
    SharpLine,
    Block,
    Ascii,
}

impl SnakeStyle {
    fn next(self) -> SnakeStyle {
        match self {
            SnakeStyle::CurvedLine => SnakeStyle::SharpLine,
            SnakeStyle::SharpLine => SnakeStyle::Block,
            SnakeStyle::Block => SnakeStyle::Ascii,
            SnakeStyle::Ascii => SnakeStyle::CurvedLine,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AppleStyle {
    Filled,
    Outline,
    Block,
    Ascii,
}

impl AppleStyle {
    fn next(self) -> AppleStyle {
        match self {
            AppleStyle::Filled => AppleStyle::Outline,
            AppleStyle::Outline => AppleStyle::Block,
            AppleStyle::Block => AppleStyle::Ascii,
            AppleStyle::Ascii => AppleStyle::Filled,
        }
    }

    fn display(self) -> char {
        match self {
            AppleStyle::Filled => '●',
            AppleStyle::Outline => '○',
            AppleStyle::Block => '█',
            AppleStyle::Ascii => 'O',
        }
    }
}

impl SegmentType {
    fn display(self, style: SnakeStyle) -> char {
        match style {
            SnakeStyle::CurvedLine => match self {
                SegmentType::NorthSouth => '│',
                SegmentType::NorthEast => '╰',
                SegmentType::NorthWest => '╯',
                SegmentType::SouthEast => '╭',
                SegmentType::SouthWest => '╮',
                SegmentType::EastWest => '─',
            },
            SnakeStyle::SharpLine => match self {
                SegmentType::NorthSouth => '│',
                SegmentType::NorthEast => '└',
                SegmentType::NorthWest => '┘',
                SegmentType::SouthEast => '┌',
                SegmentType::SouthWest => '┐',
                SegmentType::EastWest => '─',
            },
            SnakeStyle::Ascii => match self {
                SegmentType::NorthSouth => '|',
                SegmentType::NorthEast => '`',
                SegmentType::NorthWest => '`',
                SegmentType::SouthEast => '.',
                SegmentType::SouthWest => '.',
                SegmentType::EastWest => '-',
            },
            SnakeStyle::Block => '█', // All segments are blocks
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
    snake_style: SnakeStyle,
    apple_style: AppleStyle,
    wall_wrap: bool,
    color: bool,
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
            snake_style: SnakeStyle::CurvedLine,
            apple_style: AppleStyle::Filled,
            wall_wrap: false,
            color: true,
        }
    }
}

fn render_all(game: &GameState) {
    // Clear
    execute!(stdout(), Clear(ClearType::All)).unwrap();

    // Apple
    execute!(stdout(), MoveTo(game.apple.0, game.apple.1)).unwrap();
    if game.color {
        print!("{}", style(game.apple_style.display()).with(Color::Red));
    } else {
        print!("{}", game.apple_style.display());
    }

    // Snake
    for Segment(x, y, s, _) in &game.snake {
        execute!(stdout(), MoveTo(*x, *y)).unwrap();
        if game.color {
            print!("{}", style(s.display(game.snake_style)).with(Color::Green));
        } else {
            print!("{}", s.display(game.snake_style))
        }
    }

    // Board
    execute!(stdout(), MoveTo(0, game.board.1)).unwrap();
    print!(
        "{}",
        SegmentType::EastWest
            .display(game.snake_style)
            .to_string()
            .repeat(game.board.0 as usize)
    );
    for i in 0..game.board.1 {
        execute!(stdout(), MoveTo(game.board.0, i)).unwrap();
        print!(
            "{}",
            SegmentType::NorthSouth
                .display(game.snake_style)
                .to_string()
        );
    }
    execute!(stdout(), MoveTo(game.board.0, game.board.1)).unwrap();
    print!(
        "{}",
        SegmentType::NorthWest.display(game.snake_style).to_string()
    );

    // Flush
    stdout().lock().flush().unwrap();
}

fn game_over() {
    println!("\nGame Over");
}

fn handle_input(game: &Arc<Mutex<GameState>>) {
    if let Event::Key(k) = read().unwrap() {
        let mut game = game.lock().unwrap();
        match k.code {
            // Quit
            KeyCode::Char('q') => {
                execute!(stdout(), Show).unwrap();
                disable_raw_mode().unwrap();
                println!();
                std::process::exit(0);
            }

            // Up
            KeyCode::Char('k') | KeyCode::Up => {
                if game.snake[game.snake.len() - 1].3 != Direction::South {
                    game.direction = Direction::North;
                }
            }

            // Down
            KeyCode::Char('j') | KeyCode::Down => {
                if game.snake[game.snake.len() - 1].3 != Direction::North {
                    game.direction = Direction::South;
                }
            }

            // Left
            KeyCode::Char('h') | KeyCode::Left => {
                if game.snake[game.snake.len() - 1].3 != Direction::East {
                    game.direction = Direction::West;
                }
            }

            // Right
            KeyCode::Char('l') | KeyCode::Right => {
                if game.snake[game.snake.len() - 1].3 != Direction::West {
                    game.direction = Direction::East;
                }
            }

            // Decrease board x
            KeyCode::Char('1') => {
                game.board.0 = game.board.0.checked_sub(1).unwrap();
                render_all(&game);
            }

            // Increase board x
            KeyCode::Char('2') => {
                game.board.0 = game.board.0.checked_add(1).unwrap();
                render_all(&game);
            }

            // Decrease board y
            KeyCode::Char('3') => {
                game.board.1 = game.board.1.checked_sub(1).unwrap();
                render_all(&game);
            }

            // Increase board x
            KeyCode::Char('4') => {
                game.board.1 = game.board.1.checked_add(1).unwrap();
                render_all(&game);
            }

            // Decrease speed
            KeyCode::Char('5') => {
                game.delay = game.delay.checked_add(Duration::from_millis(20)).unwrap();
            }

            // Increase speed
            KeyCode::Char('6') => {
                game.delay = game.delay.checked_sub(Duration::from_millis(20)).unwrap();
            }

            // Cycle snake style
            KeyCode::Char('7') => {
                game.snake_style = game.snake_style.next();
                render_all(&game);
            }

            // Cycle apple style
            KeyCode::Char('8') => {
                game.apple_style = game.apple_style.next();
                render_all(&game);
            }

            // Toggle wall wrapping (The snake lives on a torus !!)
            KeyCode::Char('9') => {
                game.wall_wrap = !game.wall_wrap;
            }

            // Toggle color
            KeyCode::Char('0') => {
                game.color = !game.color;
                render_all(&game);
            }

            _ => {}
        }
    }
}

fn main() {
    enable_raw_mode().unwrap();
    execute!(stdout(), Hide).unwrap();

    let game = Arc::new(Mutex::new(GameState::new()));

    // Spawn input loop in another thread
    {
        let game = Arc::clone(&game);

        thread::spawn(move || loop {
            handle_input(&game);
        });
    };

    let mut rng = thread_rng();

    // Game loop
    loop {
        let head = game.lock().unwrap().head;
        let board = game.lock().unwrap().board;
        let direction = game.lock().unwrap().direction;
        let wall_wrap = game.lock().unwrap().wall_wrap;

        // New head position, based on direction
        // Wraps if collides with wall and wall_wrap is true
        // Exits loop if collides with wall and wall_wrap is false
        let new_head = match direction {
            Direction::North => {
                if head.1 > 0 {
                    (head.0, head.1 - 1)
                } else if wall_wrap {
                    (head.0, board.1 - 1)
                } else {
                    break;
                }
            }
            Direction::South => {
                if head.1 + 1 < board.1 {
                    (head.0, head.1 + 1)
                } else if wall_wrap {
                    (head.0, 0)
                } else {
                    break;
                }
            }
            Direction::West => {
                if head.0 > 0 {
                    (head.0 - 1, head.1)
                } else if wall_wrap {
                    (board.0 - 1, head.1)
                } else {
                    break;
                }
            }
            Direction::East => {
                if head.0 + 1 < board.0 {
                    (head.0 + 1, head.1)
                } else if wall_wrap {
                    (0, head.1)
                } else {
                    break;
                }
            }
        };
        {
            let mut game = game.lock().unwrap();

            // Snake contains new position, self-collision
            if game.snake.iter().any(|x| (x.0, x.1) == new_head) {
                break;
            }
            // Set head
            game.head = new_head;

            // Update second-to-last segment
            let len = game.snake.len();
            game.snake[len - 1].2 =
                SegmentType::from_next(game.snake[game.snake.len() - 1].3, game.direction);

            // New head segment
            let segment = Segment(
                new_head.0,
                new_head.1,
                SegmentType::from_dir(game.direction),
                game.direction,
            );

            // Remove oldest segment, unless you ate an apple
            if new_head == game.apple {
                // New apple position
                game.apple = (
                    rng.gen_range(0..game.board.0),
                    rng.gen_range(0..game.board.1),
                );
            } else {
                // Remove oldest segment
                game.snake.pop_front();
            }

            // Add new head segment
            game.snake.push_back(segment);

            // Render
            render_all(&game);
        }
        let delay = game.lock().unwrap().delay;
        thread::sleep(delay);
    }

    // Loop will end when game over

    // Render snake about to die
    {
        let mut game = game.lock().unwrap();
        let len = game.snake.len();
        game.snake[len - 1].2 = SegmentType::from_next(game.snake[len - 1].3, game.direction);
        render_all(&game);
    }

    game_over();

    execute!(stdout(), Show).unwrap();
    disable_raw_mode().unwrap();
    println!();
}

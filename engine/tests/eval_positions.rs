//! Table-driven regression tests for the AI's move choice on specific board
//! positions. Each case sets up a small, deliberately unambiguous position
//! (extra stones are used to break symmetry so there's exactly one correct
//! answer) and asserts either that the search always lands on a given cell,
//! or that it never does.
//!
//! To add a case: append to `CASES` below. All cases run in a single test
//! so a failure list is reported in one go rather than stopping at the first.

use pente_engine::board::BoardState;
use pente_engine::evaluation::{default_automaton, PatternScorer};
use pente_engine::search::Search;
use pente_engine::tile::{PlayerType, TileType};

#[derive(Clone, Copy)]
enum Expect {
    /// The search must land on exactly this cell.
    Always(usize, usize),
    /// The search must never land on this cell.
    Never(usize, usize),
}

struct Case {
    name: &'static str,
    width: usize,
    height: usize,
    stones: &'static [(usize, usize, TileType)],
    captures_white: u32,
    captures_black: u32,
    player: PlayerType,
    depth: usize,
    expect: Expect,
}

const CASES: &[Case] = &[
    Case {
        name: "always_wins_five_in_a_row",
        width: 15,
        height: 15,
        // White blocker at (7,1) forces the only completion to (7,6).
        stones: &[
            (7, 1, TileType::White),
            (7, 2, TileType::Black),
            (7, 3, TileType::Black),
            (7, 4, TileType::Black),
            (7, 5, TileType::Black),
        ],
        captures_white: 0,
        captures_black: 0,
        player: PlayerType::Black,
        depth: 1,
        expect: Expect::Always(7, 6),
    },
    Case {
        name: "always_takes_fifth_capture",
        width: 15,
        height: 15,
        // Black brackets White's pair at (7,3)-(7,4) by playing (7,5);
        // captures_black is one pair short of the win.
        stones: &[
            (7, 2, TileType::Black),
            (7, 3, TileType::White),
            (7, 4, TileType::White),
        ],
        captures_white: 0,
        captures_black: 4,
        player: PlayerType::Black,
        depth: 1,
        expect: Expect::Always(7, 5),
    },
    Case {
        name: "always_blocks_five_in_a_row_win",
        width: 15,
        height: 15,
        // White has four in a row, blocked on the left by Black; the only
        // way to stop White completing five is to occupy (7,6).
        stones: &[
            (7, 1, TileType::Black),
            (7, 2, TileType::White),
            (7, 3, TileType::White),
            (7, 4, TileType::White),
            (7, 5, TileType::White),
        ],
        captures_white: 0,
        captures_black: 0,
        player: PlayerType::Black,
        depth: 1,
        expect: Expect::Always(7, 6),
    },
    Case {
        name: "always_grabs_obvious_capture",
        width: 15,
        height: 15,
        // Non-winning capture (captures start at 0): still the clearly
        // correct tactical move.
        stones: &[
            (7, 2, TileType::Black),
            (7, 3, TileType::White),
            (7, 4, TileType::White),
        ],
        captures_white: 0,
        captures_black: 0,
        player: PlayerType::Black,
        depth: 0,
        expect: Expect::Always(7, 5),
    },
    Case {
        name: "always_blocks_obvious_capture",
        width: 15,
        height: 15,
        // White threatens to bracket Black's pair at (7,3)-(7,4) by playing
        // (7,5); Black must occupy (7,5) first to prevent the capture.
        stones: &[
            (7, 2, TileType::White),
            (7, 3, TileType::Black),
            (7, 4, TileType::Black),
        ],
        captures_white: 0,
        captures_black: 0,
        player: PlayerType::Black,
        depth: 1,
        expect: Expect::Always(7, 5),
    },
    Case {
        name: "always_blocks_open_three",
        width: 15,
        height: 15,
        // White has an open three at (7,4)-(7,6). Black already has a
        // stone at (6,3) directly above (7,3) (a different line, so it
        // can't be captured alongside the block), making (7,3) also build
        // a vertical pair and strictly better than blocking at (7,7).
        stones: &[
            (6, 3, TileType::Black),
            (7, 4, TileType::White),
            (7, 5, TileType::White),
            (7, 6, TileType::White),
        ],
        captures_white: 0,
        captures_black: 0,
        player: PlayerType::Black,
        depth: 1,
        expect: Expect::Always(7, 3),
    },
    Case {
        name: "always_extends_own_open_three_to_open_four",
        width: 15,
        height: 15,
        // Black has an open three at (7,4)-(7,6). White at (7,2) means
        // extending left to (7,3) would only make a closed four
        // ("211110"), while extending right to (7,7) makes a genuine open
        // four ("011110") — strictly better.
        stones: &[
            (7, 2, TileType::White),
            (7, 4, TileType::Black),
            (7, 5, TileType::Black),
            (7, 6, TileType::Black),
        ],
        captures_white: 0,
        captures_black: 0,
        player: PlayerType::Black,
        depth: 0,
        expect: Expect::Always(7, 7),
    },
];

fn find_move(case: &Case) -> (usize, usize) {
    let mut board = BoardState::new(case.width, case.height);
    for &(row, col, tile) in case.stones {
        board.set_tile(row, col, tile);
    }
    board.captures_white = case.captures_white;
    board.captures_black = case.captures_black;

    let (dfa, weights) = default_automaton();
    let search = Search::new(PatternScorer::new(dfa, weights));
    let (mv, _score) = search.find_best_move(&board, case.player, case.depth);
    mv
}

#[test]
fn eval_position_cases() {
    let mut failures = Vec::new();

    for case in CASES {
        let mv = find_move(case);
        match case.expect {
            Expect::Always(row, col) => {
                if mv != (row, col) {
                    failures.push(format!(
                        "[{}] expected move ({row},{col}), got {mv:?}",
                        case.name
                    ));
                }
            }
            Expect::Never(row, col) => {
                if mv == (row, col) {
                    failures.push(format!(
                        "[{}] expected NOT to move at ({row},{col}), but did",
                        case.name
                    ));
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} of {} case(s) failed:\n{}",
            failures.len(),
            CASES.len(),
            failures.join("\n")
        );
    }
}

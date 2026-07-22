use macroquad::prelude::*;
use pente_engine::board::BoardState;
use pente_engine::tile::{PlayerType, TileType};

#[cfg(not(target_arch = "wasm32"))]
use pente_engine::evaluation::{default_automaton, PatternScorer};
#[cfg(not(target_arch = "wasm32"))]
use pente_engine::search::Search;

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};

const BOARD_SIZE: usize = 19;
const SPRITE_PX: f32 = 16.0;

// Number of tiles across the display (board + 1-tile border on each side).
const DISPLAY_TILES: usize = BOARD_SIZE + 2;

#[cfg(not(target_arch = "wasm32"))]
const AI_DEPTH: usize = 2;

// Sprite positions as (sprite_col, sprite_row) — source notation was (row, col).
const SP_PLAYABLE: (usize, usize) = (0, 0); // row 0, col 0
const SP_GREEN: (usize, usize) = (0, 1);    // row 1, col 0
const SP_YELLOW: (usize, usize) = (1, 1);   // row 1, col 1
const SP_BORDER: (usize, usize) = (2, 1);   // row 1, col 2

enum Phase {
    Human,
    AiThinking,
    AiError,
    Over(PlayerType),
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Pente"),
        window_width: 800,
        window_height: 800,
        window_resizable: true,
        ..Default::default()
    }
}

// Returns the tile size in screen pixels and the (x, y) offset of the top-left
// board corner, so the board is centred and as large as possible.
fn layout() -> (f32, f32, f32) {
    let sw = screen_width();
    let sh = screen_height();
    let tile = (sw / DISPLAY_TILES as f32).min(sh / DISPLAY_TILES as f32);
    let ox = (sw - tile * DISPLAY_TILES as f32) / 2.0;
    let oy = (sh - tile * DISPLAY_TILES as f32) / 2.0;
    (tile, ox, oy)
}

fn draw_sprite(sprites: &Texture2D, (sc, sr): (usize, usize), x: f32, y: f32, tile: f32) {
    draw_texture_ex(
        sprites,
        x,
        y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(tile, tile)),
            source: Some(Rect::new(
                sc as f32 * SPRITE_PX,
                sr as f32 * SPRITE_PX,
                SPRITE_PX,
                SPRITE_PX,
            )),
            ..Default::default()
        },
    );
}

fn draw_board(sprites: &Texture2D, board: &BoardState, ox: f32, oy: f32, tile: f32) {
    let total = DISPLAY_TILES;
    for dr in 0..total {
        for dc in 0..total {
            let x = ox + dc as f32 * tile;
            let y = oy + dr as f32 * tile;
            let on_border = dr == 0 || dr == total - 1 || dc == 0 || dc == total - 1;
            if on_border {
                draw_sprite(sprites, SP_BORDER, x, y, tile);
            } else {
                draw_sprite(sprites, SP_PLAYABLE, x, y, tile);
                match board.get_tile(dr - 1, dc - 1) {
                    TileType::Black => draw_sprite(sprites, SP_GREEN, x, y, tile),
                    TileType::White => draw_sprite(sprites, SP_YELLOW, x, y, tile),
                    TileType::Empty => {}
                }
            }
        }
    }
}

fn screen_to_board(mx: f32, my: f32, ox: f32, oy: f32, tile: f32) -> Option<(usize, usize)> {
    let col = ((mx - ox) / tile) as isize - 1;
    let row = ((my - oy) / tile) as isize - 1;
    if col >= 0 && col < BOARD_SIZE as isize && row >= 0 && row < BOARD_SIZE as isize {
        Some((row as usize, col as usize))
    } else {
        None
    }
}

fn five_in_a_row(board: &BoardState) -> Option<PlayerType> {
    const DIRS: [(isize, isize); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
    for row in 0..board.height {
        for col in 0..board.width {
            let tile = board.get_tile(row, col);
            if tile == TileType::Empty {
                continue;
            }
            let player = match tile {
                TileType::Black => PlayerType::Black,
                TileType::White => PlayerType::White,
                _ => continue,
            };
            'dir: for &(dr, dc) in &DIRS {
                for i in 1..5isize {
                    let r = row as isize + dr * i;
                    let c = col as isize + dc * i;
                    if r < 0
                        || r >= board.height as isize
                        || c < 0
                        || c >= board.width as isize
                    {
                        continue 'dir;
                    }
                    if board.get_tile(r as usize, c as usize) != tile {
                        continue 'dir;
                    }
                }
                return Some(player);
            }
        }
    }
    None
}

fn check_win(board: &BoardState) -> Option<PlayerType> {
    if board.captures_black >= 5 {
        return Some(PlayerType::Black);
    }
    if board.captures_white >= 5 {
        return Some(PlayerType::White);
    }
    five_in_a_row(board)
}

fn encode_board(board: &BoardState) -> String {
    let mut s = format!("{}x{}:", board.width, board.height);
    for tile in &board.tiles {
        s.push(match tile {
            TileType::Empty => '.',
            TileType::Black => 'b',
            TileType::White => 'w',
        });
    }
    s
}

fn url_encode(s: &str) -> String {
    s.replace(':', "%3A")
}

#[cfg(not(target_arch = "wasm32"))]
fn make_ai_move_sync(board: &mut BoardState) -> Option<PlayerType> {
    let (dfa, weights) = default_automaton();
    let search = Search::new(PatternScorer::new(dfa, weights));
    let ((row, col), _) = search.find_best_move(board, PlayerType::White, AI_DEPTH);
    board.apply_move(row, col, PlayerType::White);
    check_win(board)
}

fn draw_message(msg: &str, ox: f32, oy: f32, board_px: f32) {
    const FONT_SIZE: f32 = 24.0;
    const PAD: f32 = 12.0;
    let dims = measure_text(msg, None, FONT_SIZE as u16, 1.0);
    let tx = ox + (board_px - dims.width) / 2.0;
    let ty = oy + board_px / 2.0;
    draw_rectangle(
        tx - PAD,
        ty - dims.height - PAD,
        dims.width + PAD * 2.0,
        dims.height + PAD * 2.0,
        Color::new(0.0, 0.0, 0.0, 0.85),
    );
    draw_text(msg, tx, ty, FONT_SIZE, WHITE);
}

#[macroquad::main(window_conf)]
async fn main() {
    let sprites = load_texture("assets/sprites.png").await.unwrap_or_else(|e| {
        // Fallback so the title at least renders even if sprites are missing.
        eprintln!("sprites load error: {e:?}");
        Texture2D::empty()
    });
    sprites.set_filter(FilterMode::Nearest);

    let mut board = BoardState::new(BOARD_SIZE, BOARD_SIZE);
    let mut phase = Phase::Human;

    #[cfg(target_arch = "wasm32")]
    let ai_channel: Arc<Mutex<Option<Result<(usize, usize), ()>>>> =
        Arc::new(Mutex::new(None));

    loop {
        let (tile, ox, oy) = layout();
        let board_px = tile * DISPLAY_TILES as f32;

        // --- Input ---
        match phase {
            Phase::Human => {
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mx, my) = mouse_position();
                    if let Some((row, col)) = screen_to_board(mx, my, ox, oy, tile) {
                        if board.get_tile(row, col) == TileType::Empty {
                            board.apply_move(row, col, PlayerType::Black);
                            match check_win(&board) {
                                Some(w) => phase = Phase::Over(w),
                                None => {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let channel = ai_channel.clone();
                                        *channel.lock().unwrap() = None;
                                        let url = format!(
                                            "/api/opponent-move?game={}&depth=0",
                                            url_encode(&encode_board(&board))
                                        );
                                        macroquad::experimental::coroutines::start_coroutine(
                                            async move {
                                                let result = match load_file(&url).await {
                                                    Ok(bytes) => {
                                                        serde_json::from_slice::<serde_json::Value>(
                                                            &bytes,
                                                        )
                                                        .ok()
                                                        .and_then(|v| {
                                                            let r = v["moves"][0]["row"].as_u64()?;
                                                            let c = v["moves"][0]["col"].as_u64()?;
                                                            Some((r as usize, c as usize))
                                                        })
                                                        .ok_or(())
                                                    }
                                                    Err(_) => Err(()),
                                                };
                                                *channel.lock().unwrap() = Some(result);
                                            },
                                        );
                                        phase = Phase::AiThinking;
                                    }

                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        let winner = make_ai_move_sync(&mut board);
                                        phase = winner.map(Phase::Over).unwrap_or(Phase::Human);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Phase::AiThinking => {
                #[cfg(target_arch = "wasm32")]
                match ai_channel.lock().unwrap().take() {
                    Some(Ok((row, col))) => {
                        board.apply_move(row, col, PlayerType::White);
                        phase = check_win(&board).map(Phase::Over).unwrap_or(Phase::Human);
                    }
                    Some(Err(())) => phase = Phase::AiError,
                    None => {}
                }
            }

            Phase::Over(_) | Phase::AiError => {}
        }

        if is_key_pressed(KeyCode::R) {
            board = BoardState::new(BOARD_SIZE, BOARD_SIZE);
            phase = Phase::Human;
            #[cfg(target_arch = "wasm32")]
            {
                *ai_channel.lock().unwrap() = None;
            }
        }

        // --- Render ---
        clear_background(BLACK);

        draw_text("PENTE", 10.0, 24.0, 24.0, WHITE);

        draw_board(&sprites, &board, ox, oy, tile);

        match &phase {
            Phase::AiThinking => draw_message("AI is thinking...", ox, oy, board_px),
            Phase::AiError => draw_message("AI unavailable  Press R to reset", ox, oy, board_px),
            Phase::Over(w) => draw_message(
                match w {
                    PlayerType::Black => "You win!  Press R to restart",
                    PlayerType::White => "AI wins!  Press R to restart",
                },
                ox,
                oy,
                board_px,
            ),
            Phase::Human => {}
        }

        next_frame().await;
    }
}

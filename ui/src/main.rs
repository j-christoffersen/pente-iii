use macroquad::prelude::*;
use pente_engine::board::BoardState;
use pente_engine::tile::{PlayerType, TileType};

const BG_VERT: &str = r#"
#version 100
attribute vec3 position;
attribute vec2 texcoord;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
}
"#;

const BG_FRAG: &str = r#"
#version 100
precision highp float;
uniform vec2 resolution;
uniform float time;

#define SPIN_ROTATION -2.0
#define SPIN_SPEED 7.0
#define CONTRAST 3.5
#define LIGHTING 0.4
#define SPIN_AMOUNT 0.25
#define PIXEL_FILTER 745.0
#define SPIN_EASE 1.0

#define COLOUR_1 vec4(0.18, 0.78, 0.26, 1.0)
#define COLOUR_2 vec4(0.04, 0.30, 0.10, 1.0)
#define COLOUR_3 vec4(0.02, 0.08, 0.03, 1.0)

vec4 effect(vec2 screenSize, vec2 screen_coords) {
    float pixel_size = length(screenSize) / PIXEL_FILTER;
    vec2 uv = (floor(screen_coords * (1.0 / pixel_size)) * pixel_size
               - 0.5 * screenSize) / length(screenSize);
    float uv_len = length(uv);

    float speed = SPIN_ROTATION * SPIN_EASE * 0.2 + 302.2;
    float new_pixel_angle = atan(uv.y, uv.x) + speed
        - SPIN_EASE * 20.0 * (SPIN_AMOUNT * uv_len + (1.0 - SPIN_AMOUNT));
    vec2 mid = screenSize / length(screenSize) * 0.5;
    uv = vec2(uv_len * cos(new_pixel_angle) + mid.x,
              uv_len * sin(new_pixel_angle) + mid.y) - mid;

    uv *= 30.0;
    speed = time * SPIN_SPEED;
    vec2 uv2 = vec2(uv.x + uv.y);

    for (int i = 0; i < 5; i++) {
        uv2 += sin(max(uv.x, uv.y)) + uv;
        uv  += 0.5 * vec2(cos(5.1123314 + 0.353 * uv2.y + speed * 0.131121),
                           sin(uv2.x - 0.113 * speed));
        uv  -= cos(uv.x + uv.y) - sin(uv.x * 0.711 - uv.y);
    }

    float contrast_mod = 0.25 * CONTRAST + 0.5 * SPIN_AMOUNT + 1.2;
    float paint_res = min(2.0, max(0.0, length(uv) * 0.035 * contrast_mod));
    float c1p = max(0.0, 1.0 - contrast_mod * abs(1.0 - paint_res));
    float c2p = max(0.0, 1.0 - contrast_mod * abs(paint_res));
    float c3p = 1.0 - min(1.0, c1p + c2p);
    float light = (LIGHTING - 0.2) * max(c1p * 5.0 - 4.0, 0.0)
                + LIGHTING * max(c2p * 5.0 - 4.0, 0.0);

    return (0.3 / CONTRAST) * COLOUR_1
         + (1.0 - 0.3 / CONTRAST) * (COLOUR_1 * c1p + COLOUR_2 * c2p
             + vec4(c3p * COLOUR_3.rgb, c3p * COLOUR_1.a))
         + light;
}

void main() {
    gl_FragColor = effect(resolution, gl_FragCoord.xy);
}
"#;

#[cfg(not(target_arch = "wasm32"))]
use pente_engine::evaluation::{default_automaton, PatternScorer};
#[cfg(not(target_arch = "wasm32"))]
use pente_engine::search::Search;

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};

const BOARD_SIZE: usize = 19;
const SPRITE_PX: f32 = 16.0;
const BORDER: usize = 2; // 2-tile-wide frame on every side
const DISPLAY_TILES: usize = BOARD_SIZE + BORDER * 2; // 23

#[cfg(not(target_arch = "wasm32"))]
const AI_DEPTH: usize = 2;

// Sprite constants: (sprite_col, sprite_row) = (col, row) in the 16-px grid.
// User coordinates are (row, col); swapped here so x = col*16, y = row*16.
const SP_NORMAL: (usize, usize) = (0, 2);   // user (2,0)
const SP_SPECIAL: (usize, usize) = (1, 2);  // user (2,1)

const SP_GREEN: (usize, usize) = (0, 4);    // user (4,0)
const SP_YELLOW: (usize, usize) = (1, 4);   // user (4,1)

const SP_OUTER_CORNER: (usize, usize) = (2, 2);                            // user (2,2)
const SP_OUTER_EDGE_LEFT: [(usize, usize); 3] = [(3, 2), (4, 2), (5, 2)];      // user (2,3) (2,4) (2,5)
const SP_OUTER_EDGE_RIGHT: [(usize, usize); 3] = [(0,5), (1, 5), (2, 5)];      // user (2,0) (2,1) (2,2)
const SP_INNER_CORNER: (usize, usize) = (3, 3);                            // user (3,0)
const SP_INNER_EDGE_LEFT: [(usize, usize); 3] = [(4, 3), (5, 3), (6, 3)];      // user (3,4) (3,5) (3,6)
const SP_INNER_EDGE_RIGHT: [(usize, usize); 3] = [(0, 6), (1, 6), (2, 6)];      // user (6,0) (6,1) (6,2)
const SP_BORDER_CENTER: [(usize, usize); 4] = [(3, 4), (4,4), (5, 4), (6, 4)];    // user (3,4) (4,4) (5,4) (6,4)

enum Phase {
    Human,
    AiThinking,
    AiError,
    Over(PlayerType),
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Pente"),
        window_resizable: true,
        ..Default::default()
    }
}

fn layout() -> (f32, f32, f32) {
    let sw = screen_width();
    let sh = screen_height();
    // largest integer pixel-art scale that fits the shorter dimension
    let scale = ((sw.min(sh) / (DISPLAY_TILES as f32 * SPRITE_PX)).floor() as usize).max(1);
    let tile = SPRITE_PX * scale as f32;
    let board_px = tile * DISPLAY_TILES as f32;
    let ox = ((sw - board_px) / 2.0).floor();
    let oy = ((sh - board_px) / 2.0).floor();
    (tile, ox, oy)
}

fn draw_sprite(sprites: &Texture2D, (sc, sr): (usize, usize), x: f32, y: f32, tile: f32) {
    draw_sprite_rot(sprites, (sc, sr), x, y, tile, 0.0);
}

fn draw_sprite_rot(
    sprites: &Texture2D,
    (sc, sr): (usize, usize),
    x: f32,
    y: f32,
    tile: f32,
    rotation: f32,
) {
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
            rotation,
            pivot: Some(vec2(x + tile / 2.0, y + tile / 2.0)),
            ..Default::default()
        },
    );
}

// Special board positions: center, ±6 cardinal, ±3 and ±6 diagonal.
fn is_special(row: usize, col: usize) -> bool {
    let c = BOARD_SIZE / 2; // 9
    let dr = row as isize - c as isize;
    let dc = col as isize - c as isize;
    if dr == 0 && dc == 0 { return true; }
    if dr == 0 && (dc == 6 || dc == -6) { return true; }
    if dc == 0 && (dr == 6 || dr == -6) { return true; }
    dr.abs() == dc.abs() && (dr.abs() == 3 || dr.abs() == 6)
}

// Pick the edge sprite at position `pos` (1..last) along any edge.
// Positions below mid use the left array, above mid use the right array, at mid use center.
// left_offset: (3 - left_start % 3) % 3  so index 0 falls on the first tile after the corner
// right_offset: (3 - (mid+1) % 3) % 3 — same logic from the other corner; mid+1=12, 12%3=0 → always 0
fn edge_sprite(
    pos: usize,
    mid: usize,
    left: &[(usize, usize); 3],
    center: (usize, usize),
    right: &[(usize, usize); 3],
    left_offset: usize,
    right_offset: usize,
) -> (usize, usize) {
    if pos == mid {
        center
    } else if pos < mid {
        left[(pos + left_offset) % 3]
    } else {
        right[(pos + right_offset) % 3]
    }
}

fn draw_board(sprites: &Texture2D, board: &BoardState, ox: f32, oy: f32, tile: f32) {
    use std::f32::consts::{FRAC_PI_2, PI};
    let last = DISPLAY_TILES - 1; // 22
    let mid = DISPLAY_TILES / 2;  // 11

    for dr in 0..DISPLAY_TILES {
        for dc in 0..DISPLAY_TILES {
            let x = ox + dc as f32 * tile;
            let y = oy + dr as f32 * tile;

            let outer_row = dr == 0 || dr == last;
            let outer_col = dc == 0 || dc == last;
            let inner_row = dr == 1 || dr == last - 1;
            let inner_col = dc == 1 || dc == last - 1;

            if outer_row || outer_col {
                if outer_row && outer_col {
                    let rot = match (dr == 0, dc == 0) {
                        (true,  true)  => 0.0,
                        (true,  false) => FRAC_PI_2,
                        (false, false) => PI,
                        (false, true)  => 3.0 * FRAC_PI_2,
                    };
                    draw_sprite_rot(sprites, SP_OUTER_CORNER, x, y, tile, rot);
                } else if dr == 0 {
                    let spr = edge_sprite(dc, mid, &SP_OUTER_EDGE_LEFT, SP_BORDER_CENTER[3], &SP_OUTER_EDGE_RIGHT, 2, 2);
                    draw_sprite_rot(sprites, spr, x, y, tile, 0.0);
                } else if dr == last {
                    let spr = edge_sprite(last - dc, mid, &SP_OUTER_EDGE_LEFT, SP_BORDER_CENTER[3], &SP_OUTER_EDGE_RIGHT, 2, 2);
                    draw_sprite_rot(sprites, spr, x, y, tile, PI);
                } else if dc == last {
                    let spr = edge_sprite(dr, mid, &SP_OUTER_EDGE_LEFT, SP_BORDER_CENTER[3], &SP_OUTER_EDGE_RIGHT, 2, 2);
                    draw_sprite_rot(sprites, spr, x, y, tile, FRAC_PI_2);
                } else {
                    let spr = edge_sprite(last - dr, mid, &SP_OUTER_EDGE_LEFT, SP_BORDER_CENTER[3], &SP_OUTER_EDGE_RIGHT, 2, 2);
                    draw_sprite_rot(sprites, spr, x, y, tile, 3.0 * FRAC_PI_2);
                }
            } else if inner_row || inner_col {
                if inner_row && inner_col {
                    let rot = match (dr == 1, dc == 1) {
                        (true,  true)  => 0.0,
                        (true,  false) => FRAC_PI_2,
                        (false, false) => PI,
                        (false, true)  => 3.0 * FRAC_PI_2,
                    };
                    draw_sprite_rot(sprites, SP_INNER_CORNER, x, y, tile, rot);
                } else if dr == 1 {
                    let spr = edge_sprite(dc, mid, &SP_INNER_EDGE_LEFT, SP_BORDER_CENTER[2], &SP_INNER_EDGE_RIGHT, 1, 0);
                    draw_sprite_rot(sprites, spr, x, y, tile, 0.0);
                } else if dr == last - 1 {
                    let spr = edge_sprite(last - dc, mid, &SP_INNER_EDGE_LEFT, SP_BORDER_CENTER[2], &SP_INNER_EDGE_RIGHT, 1, 0);
                    draw_sprite_rot(sprites, spr, x, y, tile, PI);
                } else if dc == last - 1 {
                    let spr = edge_sprite(dr, mid, &SP_INNER_EDGE_LEFT, SP_BORDER_CENTER[2], &SP_INNER_EDGE_RIGHT, 1, 0);
                    draw_sprite_rot(sprites, spr, x, y, tile, FRAC_PI_2);
                } else {
                    let spr = edge_sprite(last - dr, mid, &SP_INNER_EDGE_LEFT, SP_BORDER_CENTER[2], &SP_INNER_EDGE_RIGHT, 1, 0);
                    draw_sprite_rot(sprites, spr, x, y, tile, 3.0 * FRAC_PI_2);
                }
            } else {
                let br = dr - BORDER;
                let bc = dc - BORDER;
                let base = if is_special(br, bc) { SP_SPECIAL } else { SP_NORMAL };
                draw_sprite(sprites, base, x, y, tile);
                match board.get_tile(br, bc) {
                    TileType::Black => draw_sprite(sprites, SP_GREEN, x, y, tile),
                    TileType::White => draw_sprite(sprites, SP_YELLOW, x, y, tile),
                    TileType::Empty => {}
                }
            }
        }
    }
}

fn screen_to_board(mx: f32, my: f32, ox: f32, oy: f32, tile: f32) -> Option<(usize, usize)> {
    let col = ((mx - ox) / tile) as isize - BORDER as isize;
    let row = ((my - oy) / tile) as isize - BORDER as isize;
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
    let sprites = load_texture("assets/sprites-3.png").await.unwrap_or_else(|e| {
        eprintln!("sprites load error: {e:?}");
        Texture2D::empty()
    });
    sprites.set_filter(FilterMode::Nearest);

    let bg_material = load_material(
        ShaderSource::Glsl { vertex: BG_VERT, fragment: BG_FRAG },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("resolution", UniformType::Float2),
                UniformDesc::new("time", UniformType::Float1),
            ],
            ..Default::default()
        },
    );
    if let Err(ref e) = bg_material {
        eprintln!("bg shader error: {e:?}");
    }

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
        clear_background(Color::from_hex(0x131F15));

        match &bg_material {
            Ok(mat) => {
                mat.set_uniform("resolution", (screen_width(), screen_height()));
                mat.set_uniform("time", get_time() as f32);
                gl_use_material(mat);
                draw_rectangle(0.0, 0.0, screen_width(), screen_height(), WHITE);
                gl_use_default_material();
            }
            Err(_) => {}
        }

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

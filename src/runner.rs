use crate::physics::Body;
use crate::physics::Physics;
// use crate::physics::Collider;
use crate::physics::Coin;
use crate::physics::Collectible;
use crate::physics::Collider;
use crate::physics::Dynamic;
use crate::physics::Entity;
use crate::physics::Obstacle;
use crate::physics::Player;
use crate::physics::Power;

use crate::proceduralgen;
use crate::proceduralgen::ProceduralGen;
use crate::proceduralgen::TerrainSegment;

use crate::rect;

use inf_runner::Game;
use inf_runner::GameState;
use inf_runner::GameStatus;
use inf_runner::ObstacleType;
use inf_runner::PowerType;
use inf_runner::SDLCore;
use inf_runner::StaticObject;
use inf_runner::TerrainType;

use std::thread::sleep;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::rect::Rect;
// use sdl2::render::Texture;
use sdl2::render::TextureQuery;

use rand::distributions::Distribution;
use rand::distributions::Standard;
use rand::Rng;

const FPS: f64 = 60.0;
const FRAME_TIME: f64 = 1.0 / FPS as f64;

const CAM_H: u32 = 720;
const CAM_W: u32 = 1280;
pub const TILE_SIZE: u32 = 100;

// Background sine wave stuff
const IND_BACKGROUND_MID: usize = 0;
const IND_BACKGROUND_BACK: usize = 1;
const BG_CURVES_SIZE: usize = CAM_W as usize / 10;
// const BUFF_LENGTH: usize = CAM_W as usize / 4;

// Bounds to keep the player within
// Used for camera postioning
const PLAYER_UPPER_BOUND: i32 = 2 * TILE_SIZE as i32;
const PLAYER_LOWER_BOUND: i32 = CAM_H as i32 - PLAYER_UPPER_BOUND;
const PLAYER_LEFT_BOUND: i32 = TILE_SIZE as i32;
const PLAYER_RIGHT_BOUND: i32 = (CAM_W / 2) as i32 - (TILE_SIZE / 2) as i32; // More restrictve:
                                                                             // player needs space to react

/* Minimum speed player can move.
 * In actuality, the minimum distance everything moves left relative to the
 * player per iteration of the game loop. Physics Team, please change or
 * remove this as needed. 1 is just an arbitrary small number.
 */
const MIN_SPEED: i32 = 1;

// Max total number of coins, obstacles, and powers that can exist at
// once. Could be split up later for more complicated procgen
const MAX_NUM_OBJECTS: i32 = 10;

pub struct Runner;

impl Game for Runner {
    fn init() -> Result<Self, String> {
        Ok(Runner {})
    }

    fn run(&mut self, core: &mut SDLCore) -> Result<GameState, String> {
        core.wincan.set_blend_mode(sdl2::render::BlendMode::Blend);
        let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

        // Font
        let mut font = ttf_context.load_font("./assets/DroidSansMono.ttf", 128)?;
        font.set_style(sdl2::ttf::FontStyle::BOLD);

        // Load in all textures
        let texture_creator = core.wincan.texture_creator();
        let tex_bg = texture_creator.load_texture("assets/bg.png")?;
        let tex_sky = texture_creator.load_texture("assets/sky.png")?;
        let tex_grad = texture_creator.load_texture("assets/sunset_gradient.png")?;
        let tex_statue = texture_creator.load_texture("assets/statue.png")?;
        let tex_coin = texture_creator.load_texture("assets/coin.png")?;
        let tex_speed = texture_creator.load_texture("assets/speed.png")?;
        let tex_multiplier = texture_creator.load_texture("assets/multiplier.png")?;
        let tex_bouncy = texture_creator.load_texture("assets/bouncy.png")?;
        let tex_floaty = texture_creator.load_texture("assets/floaty.png")?;
        let tex_shield = texture_creator.load_texture("assets/shield.png")?;
        let tex_shielded = texture_creator.load_texture("assets/shielded_player.png")?;

        let tex_resume = texture_creator
            .create_texture_from_surface(
                &font
                    .render("Escape/Space - Resume Play")
                    .blended(Color::RGBA(119, 3, 252, 255))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

        let tex_restart = texture_creator
            .create_texture_from_surface(
                &font
                    .render("R - Restart game")
                    .blended(Color::RGBA(119, 3, 252, 255))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

        let tex_main = texture_creator
            .create_texture_from_surface(
                &font
                    .render("M - Main menu")
                    .blended(Color::RGBA(119, 3, 252, 255))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

        let tex_quit = texture_creator
            .create_texture_from_surface(
                &font
                    .render("Q - Quit game")
                    .blended(Color::RGBA(119, 3, 252, 255))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

        // Create player at default position
        let mut player = Player::new(
            rect!(
                CAM_W / 2 - TILE_SIZE / 2, // Center of screen
                CAM_H / 2 - TILE_SIZE / 2,
                TILE_SIZE,
                TILE_SIZE
            ),
            3.0,
            texture_creator.load_texture("assets/player.png")?,
        );
        let mut active_power: Option<PowerType> = None;
        let mut power_timer: i32 = 0; // Current powerup expires when it reaches 0
        let mut coin_count: i32 = 0; // Total num coins collected

        // Initialize ground / object vectors
        let mut curr_num_objects: i32 = 0;
        let mut all_terrain: Vec<TerrainSegment> = Vec::new();
        let mut all_obstacles: Vec<Obstacle> = Vec::new();
        let mut all_coins: Vec<Coin> = Vec::new();
        let mut all_powers: Vec<Power> = Vec::new(); // Refers to powers currently spawned on the
                                                     // ground, not active powers

        // Used to keep track of animation status
        let mut player_anim: i32 = 0; // 4 frames of animation
        let mut coin_anim: i32 = 0; // 60 frames of animation

        // Score of an entire run
        let mut total_score: i32 = 0;

        let mut game_paused: bool = false;
        let mut initial_pause: bool = false;
        let mut game_over: bool = false;

        // Seems out of place?
        let mut shielded = false;

        // Number of frames to delay the end of the game by for demonstrating player
        // collision this should be removed once the camera tracks the player
        // properly
        let mut game_over_timer = 120;

        // FPS tracking
        let mut all_frames: i32 = 0;
        let mut last_raw_time;
        let mut last_measurement_time = Instant::now();

        // Used to transition to credits or back to title screen
        let mut next_status = GameStatus::Main;

        // Object spawning vars
        // let mut object_spawn: usize = 0;
        // let mut object_count: i32 = 0;
        let mut spawn_timer: i32 = 500; // Can spawn a new object when it reaches 0
        let mut min_spawn_gap: i32 = 500; // Value spawn_timer is reset to upon spawning
                                          // an object. Decreases over time.

        // Physics vars
        let mut player_accel_rate: f64 = -10.0;
        let mut player_jump_change: f64 = 0.0;
        let mut player_speed_adjust: f64 = 0.0;

        // Background & sine wave vars
        let mut bg_buff = 0;
        let mut bg_tick = 0;
        let mut buff_1: usize = 0;
        let mut buff_2: usize = 0;
        // Perlin noise curves the player can't interact with, for visuals only
        // Use IND_BACKGROUND_BACK and IND_BACKGROUND_MID
        let mut background_curves: [[i16; BG_CURVES_SIZE]; 2] = [[0; BG_CURVES_SIZE]; 2];

        // Rand thread to be utilized within runner
        let mut rng = rand::thread_rng();

        // Frequency control modifier for background sine waves
        let freq: f32 = rng.gen::<f32>() * 1000.0 + 100.0;

        // Amplitude control modifiers for background sine waves
        let amp_1: f32 = rng.gen::<f32>() * 4.0 + 1.0;
        let amp_2: f32 = rng.gen::<f32>() * 2.0 + amp_1;

        // Perlin Noise init
        let mut random: [[(i32, i32); 256]; 256] = [[(0, 0); 256]; 256];
        for i in 0..random.len() - 1 {
            for j in 0..random.len() - 1 {
                random[i][j] = (rng.gen_range(0..256), rng.gen_range(0..256));
            }
        }

        // Initialize the starting terrain segments
        let p0 = (0.0, (CAM_H / 3) as f64);
        all_terrain.push(ProceduralGen::gen_terrain(
            &random,
            p0,
            CAM_W as i32,
            CAM_H as i32,
            false,
            false,
            false,
        ));
        all_terrain.push(ProceduralGen::gen_terrain(
            &random,
            (
                0.0,
                all_terrain[0].curve()[all_terrain[0].curve().len() - 2].1 as f64,
            ),
            CAM_W as i32,
            CAM_H as i32,
            false,
            false,
            false,
        ));

        // Pre-Generate perlin curves for background hills
        for i in 0..BG_CURVES_SIZE {
            background_curves[IND_BACKGROUND_MID][i] =
                proceduralgen::gen_perlin_hill_point((i + buff_1), freq, amp_1, 0.5, 600.0);
            background_curves[IND_BACKGROUND_BACK][i] =
                proceduralgen::gen_perlin_hill_point((i + buff_2), freq, amp_2, 1.0, 820.0);
        }

        /* ~~~~~~ Main Game Loop ~~~~~~ */
        'gameloop: loop {
            last_raw_time = Instant::now(); // FPS tracking

            // Score collected in a single iteration of the game loop
            let mut curr_step_score: i32 = 0;

            /* ~~~~~~ Pausing Handler ~~~~~~ */
            if game_paused {
                for event in core.event_pump.poll_iter() {
                    match event {
                        Event::Quit { .. }
                        | Event::KeyDown {
                            keycode: Some(Keycode::Q),
                            ..
                        } => {
                            next_status = GameStatus::Credits;
                            break 'gameloop;
                        }
                        Event::KeyDown {
                            keycode: Some(k), ..
                        } => match k {
                            Keycode::Escape | Keycode::Space => {
                                game_paused = false;
                            }
                            Keycode::R => {
                                next_status = GameStatus::Game;
                                break 'gameloop;
                            }
                            Keycode::M => {
                                next_status = GameStatus::Main;
                                break 'gameloop;
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                } // End Loop

                // Draw pause screen once due to BlendMode setting
                if initial_pause {
                    // Pause screen background, semitransparent grey
                    core.wincan.set_draw_color(Color::RGBA(0, 0, 0, 128));
                    core.wincan.fill_rect(rect!(0, 0, CAM_W, CAM_H))?;

                    // Draw pause screen text
                    core.wincan
                        .copy(&tex_resume, None, Some(rect!(100, 100, 1000, 125)))?;
                    core.wincan
                        .copy(&tex_restart, None, Some(rect!(100, 250, 700, 125)))?;
                    core.wincan
                        .copy(&tex_main, None, Some(rect!(100, 400, 600, 125)))?;
                    core.wincan
                        .copy(&tex_quit, None, Some(rect!(100, 550, 600, 125)))?;

                    core.wincan.present();
                    initial_pause = false;
                }
            }
            // Normal unpaused game state
            else {
                // End game loop, 'player has lost' state
                if game_over {
                    game_over_timer -= 1; // Animation buffer
                    if game_over_timer == 0 {
                        break 'gameloop;
                    }
                }

                let curr_ground_point: Point = get_ground_coord(&all_terrain, player.x());
                let next_ground_point: Point =
                    get_ground_coord(&all_terrain, player.x() + TILE_SIZE as i32);
                let angle = ((next_ground_point.y() as f64 - curr_ground_point.y() as f64)
                    / (TILE_SIZE as f64))
                    .atan();

                /* ~~~~~~ Handle Input ~~~~~~ */
                for event in core.event_pump.poll_iter() {
                    match event {
                        Event::Quit { .. } => break 'gameloop,
                        Event::KeyDown {
                            keycode: Some(k), ..
                        } => match k {
                            Keycode::W | Keycode::Up | Keycode::Space => {
                                if player.is_jumping() {
                                    player.resume_flipping();
                                } else {
                                    player.jump(curr_ground_point, true, player_jump_change);
                                }
                            }
                            Keycode::Escape => {
                                game_paused = true;
                                initial_pause = true;
                            }
                            _ => {}
                        },
                        Event::KeyUp {
                            keycode: Some(k), ..
                        } => match k {
                            Keycode::W | Keycode::Up | Keycode::Space => {
                                player.stop_flipping();
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }

                /* ~~~~~~ Handle Player Collecting an Object ~~~~~~ */
                /* ~~~~~~ Is it actually Handle Player Collisions? ~~~~~~ */

                // Add back obstacle collisions?

                // Remove coins if player collects them
                let mut to_remove_ind: i32 = -1;
                let mut counter = 0;
                for coin in all_coins.iter_mut() {
                    if Physics::check_collection(&mut player, coin) {
                        if !coin.collected() {
                            to_remove_ind = counter;
                            //so you only collect each coin once
                            coin.collect(); //deletes the coin once collected (but takes too long)
                            coin_count += 1;
                            curr_step_score += coin.value();
                        }
                        continue;
                    }
                    counter += 1;
                }
                if to_remove_ind != -1 {
                    all_coins.remove(to_remove_ind as usize);
                }

                // Remove power ups if player collects them
                // Rough, but should follow the coin idea closely.
                let mut to_remove_ind: i32 = -1;
                counter = 0;
                for power in all_powers.iter_mut() {
                    if Physics::check_power(&mut player, power) {
                        if !power.collected() {
                            to_remove_ind = counter;
                            match power.power_type {
                                Some(PowerType::SpeedBoost) => {
                                    active_power = Some(PowerType::SpeedBoost);
                                }
                                Some(PowerType::ScoreMultiplier) => {
                                    active_power = Some(PowerType::ScoreMultiplier);
                                }
                                Some(PowerType::BouncyShoes) => {
                                    active_power = Some(PowerType::BouncyShoes);
                                }
                                Some(PowerType::LowerGravity) => {
                                    active_power = Some(PowerType::LowerGravity);
                                }
                                Some(PowerType::Shield) => {
                                    active_power = Some(PowerType::Shield);
                                }
                                _ => {}
                            }

                            // Reset any previously active power values to default
                            // Shouldn't need a var to say if we're overriding a power, just do it
                            // power_override = false;
                            player_accel_rate = -10.0;
                            player_jump_change = 0.0;
                            player_speed_adjust = 0.0;
                            shielded = false;

                            power.collect();
                            power_timer = 360; // Hardcoded powerup duration
                        }
                        continue;
                    }
                    counter += 1;
                }
                if to_remove_ind != -1 {
                    all_powers.remove(to_remove_ind as usize);
                }

                /* ~~~~~~ Power Handling Section ~~~~~~ */
                if power_timer > 0 {
                    power_timer -= 1;
                    match active_power {
                        Some(PowerType::SpeedBoost) => {
                            // May not be the proper way to handle this.
                            // Adds player speed adjust to player's velocity
                            player_speed_adjust = 5.0;
                        }
                        Some(PowerType::ScoreMultiplier) => {
                            // Handled below when adding curr_step_score to
                            // total_score
                        }
                        Some(PowerType::BouncyShoes) => {
                            // Forces jumping while active and jumps 0.3 velocity units higher
                            player_jump_change = 0.3;
                            // This will need changed for refractor
                            player.jump(curr_ground_point, true, player_jump_change);
                        }
                        Some(PowerType::LowerGravity) => {
                            // Accel rate is how the y velocity is clamped
                            // Has player jump 0.2 velocity units higher.
                            player_accel_rate = -5.0;
                            player_jump_change = 0.2;
                        }
                        Some(PowerType::Shield) => {
                            // Shielded will say to ignore obstacle collisions
                            shielded = true;
                        }
                        _ => {}
                    }
                } else {
                    // power_timer = 0
                    // Reset values to default if power times out
                    match active_power {
                        // Stop any power from going
                        Some(PowerType::SpeedBoost) => {
                            player_speed_adjust = 0.0;
                        }
                        Some(PowerType::ScoreMultiplier) => {}
                        Some(PowerType::BouncyShoes) => {
                            player_jump_change = 0.0;
                        }
                        Some(PowerType::LowerGravity) => {
                            player_accel_rate = -10.0;
                            player_jump_change = 0.0;
                        }
                        Some(PowerType::Shield) => {
                            shielded = false;
                        }
                        _ => {}
                    }
                    active_power = None;
                }

                // Applies gravity, normal & friction now
                // Friciton is currently way OP (stronger than grav) bc cast to i32 in
                // apply_force so to ever have an effect, it needs to be set > 1
                // for now...
                Physics::apply_gravity(&mut player, angle, 0.3);

                //apply friction
                //Physics::apply_friction(&mut player, 1.0);

                for obs in all_obstacles.iter_mut() {
                    obs.update_vel(0.0, 0.0); // These args do nothing
                    obs.update_pos(Point::new(0, 0), 15.0, false);
                }
                player.update_pos(curr_ground_point, angle, game_over);
                player.update_vel(player_accel_rate, player_speed_adjust);
                player.flip();

                //kinematics change, scroll speed does not :(
                //can see best when super curvy map generated
                /*  println!(
                    "px:{}  vx:{} ax:{} ay:{}",
                    player.x(),
                    player.vel_x(),
                    player.accel_x(),
                    player.accel_y(),
                ); */

                if !player.collide_terrain(curr_ground_point, angle) {
                    game_over = true;
                    initial_pause = true;
                    continue;
                }

                // Generate new terrain / objects if player hasn't died
                if !game_over {
                    // Every 3 ticks, build a new front mountain segment
                    if bg_tick % 3 == 0 {
                        for i in 0..(BG_CURVES_SIZE as usize - 1) {
                            background_curves[IND_BACKGROUND_MID][i] =
                                background_curves[IND_BACKGROUND_MID][i + 1];
                        }
                        buff_1 += 1;
                        let chunk_1 = proceduralgen::gen_perlin_hill_point(
                            ((BG_CURVES_SIZE - 1) as usize + buff_1),
                            freq,
                            amp_1,
                            0.5,
                            600.0,
                        );
                        background_curves[IND_BACKGROUND_MID][(BG_CURVES_SIZE - 1) as usize] =
                            chunk_1;
                    }

                    // Every 5 ticks, build a new back mountain segment
                    if bg_tick % 5 == 0 {
                        for i in 0..(BG_CURVES_SIZE as usize - 1) {
                            background_curves[IND_BACKGROUND_BACK][i] =
                                background_curves[IND_BACKGROUND_BACK][i + 1];
                        }
                        buff_2 += 1;
                        let chunk_2 = proceduralgen::gen_perlin_hill_point(
                            ((BG_CURVES_SIZE - 1) as usize + buff_2),
                            freq,
                            amp_2,
                            1.0,
                            820.0,
                        );
                        background_curves[IND_BACKGROUND_BACK][(BG_CURVES_SIZE - 1) as usize] =
                            chunk_2;
                    }

                    /* ~~~~~~ Object Generation ~~~~~~ */
                    // Decrease min_spawn_gap to increase spawn rates based on total_score
                    // These numbers could be terrible, we should mess around with it
                    if total_score > 100000 {
                        min_spawn_gap = 300; // Cap
                    } else if total_score > 90000 {
                        min_spawn_gap = 320;
                    } else if total_score > 80000 {
                        min_spawn_gap = 340;
                    } else if total_score > 70000 {
                        min_spawn_gap = 360;
                    } else if total_score > 60000 {
                        min_spawn_gap = 380;
                    } else if total_score > 50000 {
                        min_spawn_gap = 400;
                    } else if total_score > 40000 {
                        min_spawn_gap = 420;
                    } else if total_score > 30000 {
                        min_spawn_gap = 440;
                    } else if total_score > 30000 {
                        min_spawn_gap = 460;
                    } else if total_score > 10000 {
                        min_spawn_gap = 480;
                    } else {
                        min_spawn_gap = 500; // Default
                    }

                    // Choose new object to generate
                    let mut new_object: Option<StaticObject> = None;
                    let mut curr_num_objects =
                        all_obstacles.len() + all_coins.len() + all_powers.len();
                    let spawn_trigger = rng.gen_range(0..MAX_NUM_OBJECTS);
                    if spawn_timer > 0 {
                        spawn_timer -= 1;
                    } else if spawn_trigger >= curr_num_objects as i32 {
                        new_object = Some(proceduralgen::choose_static_object());
                        curr_num_objects += 1;
                        spawn_timer = min_spawn_gap;
                    } else if spawn_trigger < curr_num_objects as i32 {
                        // Min spawn gap can be replaced with basically any value for this random
                        // range. Smaller values will spawn objects more often
                        spawn_timer = rng.gen_range(0..min_spawn_gap);
                    }

                    // Spawn new object
                    // Everything is using (x,y) = (CAM_W,0) right now,
                    // but it should be using (CAM_W, curr_ground_point.y())
                    match new_object {
                        Some(StaticObject::Statue) => {
                            let obstacle = Obstacle::new(
                                rect!(CAM_W, 0, 0, 0),
                                50.0,
                                texture_creator.load_texture("assets/statue.png")?,
                                ObstacleType::Statue,
                            );
                            all_obstacles.push(obstacle);
                            // new_object = None;
                        }
                        Some(StaticObject::Coin) => {
                            let coin = Coin::new(
                                rect!(CAM_W, 0, 0, 0),
                                texture_creator.load_texture("assets/coin.png")?,
                                1000,
                            );
                            all_coins.push(coin);
                            // new_object = None;
                        }
                        Some(StaticObject::Spring) => {
                            let obstacle = Obstacle::new(
                                rect!(CAM_W, 0, 0, 0),
                                1.0,
                                texture_creator.load_texture("assets/temp_spring.jpg")?,
                                ObstacleType::Spring,
                            );
                            all_obstacles.push(obstacle);
                            // new_object = None;
                        }
                        Some(StaticObject::Power) => {
                            let pow = Power::new(
                                rect!(CAM_W, 0, 0, 0),
                                texture_creator.load_texture("assets/powerup.png")?,
                                Some(proceduralgen::choose_power_up()),
                            );
                            all_powers.push(pow);
                            // new_object = None;
                        }
                        _ => {}
                    }

                    // Update total_score
                    // Poorly placed rn, should be after postion / hitbox / collision update
                    // but before drawing
                    if !game_over {
                        match active_power {
                            Some(PowerType::ScoreMultiplier) => {
                                curr_step_score *= 2; // Hardcoded power bonus
                            }
                            _ => {}
                        }
                        total_score += curr_step_score;
                    }

                    /* Update ground / object positions to move player forward
                     * by the distance they should move this single iteration of the game loop
                     */
                    let iteration_distance: i32 = MIN_SPEED + player.vel_x() as i32;
                    for ground in all_terrain.iter_mut() {
                        ground.travel_update(iteration_distance);
                    }
                    /*  travel_update needs to be implemented in physics.rs
                        for obstacles, coins and power ups.
                        See terrain segment implementation in proceduralgen.rs,
                        it should be almost exactly the same

                    for obs in all_obstacles.iter() {
                        obs.travel_update(iteration_distance);
                    }
                    for coin in all_coins.iter() {
                        coin.travel_update(iteration_distance);
                    }
                    for powerUp in all_powers.iter() {
                        powerUp.travel_update(iteration_distance);
                    }
                    */

                    /* ~~~~~~ Begin Camera Section ~~~~~~ */
                    /* This should be the very last section of calcultions,
                     * as the camera position relies upon updated math for
                     * EVERYTHING ELSE. Below the camera section we have
                     * removal of offscreen objects from their vectors,
                     * animation updates, the drawing section, and FPS calculation only.
                     */
                    let camera_adj_x: i32 = 0;
                    let camera_adj_y: i32 = 0;

                    // Adjust camera horizontally if updated player x pos is out of bounds
                    if player.x() < PLAYER_LEFT_BOUND {
                        let camera_adj_x = PLAYER_LEFT_BOUND - player.x();
                    } else if (curr_ground_point.x() + TILE_SIZE as i32) > PLAYER_RIGHT_BOUND {
                        let camera_adj_x = PLAYER_RIGHT_BOUND - player.x();
                    }

                    // Adjust camera vertically based on y/height of the ground
                    if curr_ground_point.y() < PLAYER_UPPER_BOUND {
                        let camera_adj_y = PLAYER_UPPER_BOUND - curr_ground_point.y();
                    } else if (curr_ground_point.y() + TILE_SIZE as i32) > PLAYER_LOWER_BOUND {
                        let camera_adj_y = PLAYER_LOWER_BOUND - curr_ground_point.y();
                    }

                    // Add adjustment to terrain
                    for ground in all_terrain.iter_mut() {
                        ground.camera_adj(camera_adj_x, camera_adj_y);
                    }

                    /*  camera_adj needs to be implemented in physics.rs
                        for obstacles, coins and power ups, and the player.
                        See terrain segment implementation in proceduralgen.rs,
                        it should be almost exactly the same.

                    // Add adjustment to obstacles
                    for obs in all_obstacles.iter() {
                        obs.travel_update(iteration_distance);
                    }

                    // Add adjustment to coins
                    for coin in all_coins.iter() {
                        coin.travel_update(iteration_distance);
                    }
                    // Add adjustment to power ups
                    for powerUp in all_powers.iter() {
                        powerUp.travel_update(iteration_distance);
                    }

                    // Add adjustment to player
                    player.camera_adj(camera_adj_x, camera_adj_y);
                    */
                    /* ~~~~~~ End Camera Section ~~~~~~ */

                    /* ~~~~~~ Remove stuff which is now offscreen ~~~~~~ */
                    // Terrain
                    let mut ind: i32 = -1;
                    for ground in all_terrain.iter() {
                        if ground.x() + ground.w() <= 0 {
                            ind += 1;
                        }
                    }
                    for i in 0..ind {
                        all_terrain.remove(i as usize);
                    }

                    //  Obstacles
                    ind = -1;
                    for obs in all_obstacles.iter() {
                        if obs.x() + TILE_SIZE as i32 <= 0 {
                            ind += 1;
                        }
                    }
                    for i in 0..ind {
                        all_obstacles.remove(i as usize);
                    }

                    // Coins
                    ind = -1;
                    for coin in all_coins.iter() {
                        if coin.x() + TILE_SIZE as i32 <= 0 {
                            ind += 1;
                        }
                    }
                    for i in 0..ind {
                        all_coins.remove(i as usize);
                    }

                    // Power ups
                    ind = -1;
                    for power in all_powers.iter_mut() {
                        if power.x() + TILE_SIZE as i32 <= 0 {
                            ind += 1;
                        }
                    }
                    for i in 0..ind {
                        all_powers.remove(i as usize);
                    }

                    /* ~~~~~~ Animation Updates ~~~~~~ */
                    bg_tick += 1;

                    /* Player animation is barely visible, maybe reimplement later?
                    if bg_tick % 2 == 0 {
                        player_anim += 1;
                        player_anim %= 4;
                    }
                    */

                    // Shift background images & sine waves?
                    if bg_tick % 10 == 0 {
                        bg_buff -= 1;
                    }

                    // Reset sine wave tick (to prevent large values?)
                    if bg_tick % 3 == 0 && bg_tick % 5 == 0 {
                        bg_tick = 0;
                    }

                    // Reset background image buffer upon leftmost bg image moving completely
                    // offscreen
                    if -bg_buff == CAM_W as i32 {
                        bg_buff = 0;
                    }

                    // Next frame for coin animation
                    coin_anim += 1;
                    coin_anim %= 60;

                    /* ~~~~~~ Draw All Elements ~~~~~~ */
                    // Wipe screen every frame
                    core.wincan.set_draw_color(Color::RGBA(3, 120, 206, 255));
                    core.wincan.clear();

                    // Bottom layer of background, black skybox
                    core.wincan.set_draw_color(Color::RGBA(0, 0, 0, 255));
                    core.wincan.fill_rect(rect!(0, 470, CAM_W, CAM_H))?;

                    // Sky
                    core.wincan
                        .copy(&tex_sky, None, rect!(bg_buff, 0, CAM_W, CAM_H / 3))?;
                    core.wincan.copy(
                        &tex_sky,
                        None,
                        rect!(CAM_W as i32 + bg_buff, 0, CAM_W, CAM_H / 3),
                    )?;

                    // Sunset gradient - doesn't need to scroll left
                    core.wincan
                        .copy(&tex_grad, None, rect!(0, -128, CAM_W, CAM_H))?;

                    // Background
                    core.wincan
                        .copy(&tex_bg, None, rect!(bg_buff, -150, CAM_W, CAM_H))?;
                    core.wincan.copy(
                        &tex_bg,
                        None,
                        rect!(bg_buff + (CAM_W as i32), -150, CAM_W, CAM_H),
                    )?;

                    // Background perlin noise curves
                    for i in 0..background_curves[IND_BACKGROUND_MID].len() - 1 {
                        // Furthest back perlin noise curves
                        core.wincan.set_draw_color(Color::RGBA(128, 51, 6, 255));
                        core.wincan.fill_rect(rect!(
                            i * CAM_W as usize / BG_CURVES_SIZE
                                + CAM_W as usize / BG_CURVES_SIZE / 2,
                            CAM_H as i16 - background_curves[IND_BACKGROUND_BACK][i],
                            CAM_W as usize / BG_CURVES_SIZE,
                            CAM_H as i16
                        ))?;

                        // Midground perlin noise curves
                        core.wincan.set_draw_color(Color::RGBA(96, 161, 152, 255));
                        core.wincan.fill_rect(rect!(
                            i * CAM_W as usize / BG_CURVES_SIZE
                                + CAM_W as usize / BG_CURVES_SIZE / 2,
                            CAM_H as i16 - background_curves[IND_BACKGROUND_MID][i],
                            CAM_W as usize / BG_CURVES_SIZE,
                            CAM_H as i16
                        ))?;
                    }

                    // Active Power HUD Display
                    if active_power.is_some() {
                        match active_power {
                            Some(PowerType::SpeedBoost) => {
                                core.wincan.copy(
                                    &tex_speed,
                                    None,
                                    rect!(10, 100, TILE_SIZE, TILE_SIZE),
                                )?;
                            }
                            Some(PowerType::ScoreMultiplier) => {
                                core.wincan.copy(
                                    &tex_multiplier,
                                    None,
                                    rect!(10, 100, TILE_SIZE, TILE_SIZE),
                                )?;
                            }
                            Some(PowerType::BouncyShoes) => {
                                core.wincan.copy(
                                    &tex_bouncy,
                                    None,
                                    rect!(10, 100, TILE_SIZE, TILE_SIZE),
                                )?;
                            }
                            Some(PowerType::LowerGravity) => {
                                core.wincan.copy(
                                    &tex_floaty,
                                    None,
                                    rect!(10, 100, TILE_SIZE, TILE_SIZE),
                                )?;
                            }
                            Some(PowerType::Shield) => {
                                core.wincan.copy(
                                    &tex_shield,
                                    None,
                                    rect!(10, 100, TILE_SIZE, TILE_SIZE),
                                )?;
                            }
                            _ => {}
                        }

                        // Power duration bar
                        let m = power_timer as f64 / 360.0;
                        let r = 256.0 * (1.0 - m);
                        let g = 256.0 * (m);
                        let w = TILE_SIZE as f64 * m;
                        core.wincan.set_draw_color(Color::RGB(r as u8, g as u8, 0));
                        core.wincan.fill_rect(rect!(10, 210, w as u8, 10))?;
                    }

                    // Terrain
                    for ground in all_terrain.iter() {
                        core.wincan.set_draw_color(ground.color());
                        core.wincan.fill_rect(ground.pos())?;
                    }

                    // Set player texture
                    let mut tex_player = player.texture(); // Default
                    if shielded {
                        tex_player = &tex_shielded;
                    } /* else if ... {
                          Other player textures
                      } */

                    // Player
                    core.wincan.copy_ex(
                        tex_player,
                        rect!(player_anim * TILE_SIZE as i32, 0, TILE_SIZE, TILE_SIZE),
                        rect!(player.x(), player.y(), TILE_SIZE, TILE_SIZE),
                        player.theta() * 180.0 / std::f64::consts::PI,
                        None,
                        false,
                        false,
                    )?;
                    core.wincan.set_draw_color(Color::BLACK);

                    // Player's hitbox
                    for h in player.hitbox().iter() {
                        core.wincan.draw_rect(*h)?;
                    }

                    // Obstacles
                    for obs in all_obstacles.iter() {
                        // println!("XXXXX ypos{} vyo{} ayo{}  ", o.pos.1, o.velocity.1, o.accel.1
                        // );
                        match obs.obstacle_type {
                            ObstacleType::Statue => {
                                core.wincan.copy_ex(
                                    obs.texture(),
                                    None,
                                    rect!(obs.pos.0, obs.pos.1, TILE_SIZE, TILE_SIZE),
                                    obs.theta(),
                                    None,
                                    false,
                                    false,
                                )?;
                                core.wincan.set_draw_color(Color::RED);
                                core.wincan.draw_rect(obs.hitbox())?;
                                break;
                            }
                            ObstacleType::Spring => {
                                core.wincan.copy_ex(
                                    obs.texture(),
                                    None,
                                    rect!(obs.pos.0, obs.pos.1, TILE_SIZE, TILE_SIZE / 4),
                                    obs.theta(),
                                    None,
                                    false,
                                    false,
                                )?;
                                core.wincan.set_draw_color(Color::BLUE);
                                core.wincan.draw_rect(obs.hitbox())?;
                            }
                            _ => {}
                        }
                    }

                    // Coins
                    for coin in all_coins.iter() {
                        core.wincan.copy_ex(
                            coin.texture(),
                            rect!(coin_anim * TILE_SIZE as i32, 0, TILE_SIZE, TILE_SIZE),
                            rect!(coin.x(), coin.y(), TILE_SIZE, TILE_SIZE),
                            0.0,
                            None,
                            false,
                            false,
                        )?;
                        core.wincan.set_draw_color(Color::GREEN);
                        core.wincan.draw_rect(coin.hitbox())?;
                    }

                    // Powerups (on the ground, not active or collected)
                    for power in all_powers.iter() {
                        core.wincan.copy_ex(
                            power.texture(),
                            rect!(0, 0, TILE_SIZE, TILE_SIZE),
                            rect!(power.x(), power.y(), TILE_SIZE, TILE_SIZE),
                            0.0,
                            None,
                            false,
                            false,
                        )?;
                        core.wincan.set_draw_color(Color::YELLOW);
                        core.wincan.draw_rect(power.hitbox())?;
                    }

                    // Setup for the text of the total_score to be displayed
                    let tex_score = font
                        .render(&format!("{:08}", total_score))
                        .blended(Color::RGBA(255, 0, 0, 100))
                        .map_err(|e| e.to_string())?;

                    // Display num coins collected
                    let other_surface = font
                        .render(&format!("{:03}", coin_count))
                        .blended(Color::RGBA(100, 0, 200, 100))
                        .map_err(|e| e.to_string())?;
                    let coin_count_texture = texture_creator
                        .create_texture_from_surface(&other_surface)
                        .map_err(|e| e.to_string())?;
                    core.wincan
                        .copy(&coin_count_texture, None, Some(rect!(160, 10, 80, 50)))?;

                    // Display total_score
                    let score_texture = texture_creator
                        .create_texture_from_surface(&tex_score)
                        .map_err(|e| e.to_string())?;
                    core.wincan
                        .copy(&score_texture, None, Some(rect!(10, 10, 100, 50)))?;

                    if game_over {
                        // decrement the amount of frames until the game ends in order to
                        // demonstrate the collision
                        let game_over_texture = texture_creator
                            .create_texture_from_surface(
                                &font
                                    .render("GAME OVER")
                                    .blended(Color::RGBA(255, 0, 0, 255))
                                    .map_err(|e| e.to_string())?,
                            )
                            .map_err(|e| e.to_string())?;

                        // Cleaned up calculation of texture position
                        // Check previous versions if you want those calculations
                        core.wincan.copy(
                            &game_over_texture,
                            None,
                            Some(rect!(239, 285, 801, 149)),
                        )?;
                    }

                    core.wincan.present();
                }

                /* ~~~~~~ FPS Calculation ~~~~~~ */
                // Time taken to display the last frame
                let raw_frame_time = last_raw_time.elapsed().as_secs_f64();
                let delay = FRAME_TIME - raw_frame_time;
                // If the amount of time to display the last frame was less than expected, sleep
                // until the expected amount of time has passed
                if delay > 0.0 {
                    // Using sleep to delay will always cause slightly more delay than intended due
                    // to CPU scheduling; possibly find a better way to delay
                    sleep(Duration::from_secs_f64(delay));
                }
                all_frames += 1;
                let time_since_last_measurement = last_measurement_time.elapsed();
                // Measures the FPS once per second
                if time_since_last_measurement > Duration::from_secs(1) {
                    //println!("{} FPS", all_frames);
                    all_frames = 0;
                    last_measurement_time = Instant::now();
                }

                // The very last thing in the game loop
                // Is this some kind of physics thing that I'm too proceduralgen to understand?
                player.reset_accel();
            }

            /* ~~~~~~ Helper Functions ~~~~~ */
            // Given the current terrain and an x coordinate of the screen,
            // returns the (x, y) of the ground at that x
            fn get_ground_coord(all_terrain: &Vec<TerrainSegment>, screen_x: i32) -> Point {
                for ground in all_terrain.iter() {
                    if (screen_x >= ground.x()) & (screen_x <= ground.x() + ground.w()) {
                        let point_ind: usize = (screen_x - ground.x()) as usize;
                        return Point::new(
                            ground.curve().get(point_ind).unwrap().0,
                            ground.curve().get(point_ind).unwrap().1,
                        );
                    }
                }
                return Point::new(-1, -1);
            }
        } // End gameloop
        Ok(GameState {
            status: Some(next_status),
            score: total_score,
        })
    } // End run fn
} // End impl

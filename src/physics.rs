use inf_runner::ObstacleType;
use inf_runner::PowerType;
use inf_runner::TerrainType;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::render::Texture;

use std::time::{Duration, SystemTime};

use crate::runner::TILE_SIZE as InitTILE_SIZE;
use std::f64::consts::PI;

const LOWER_SPEED: f64 = -5.0;
const UPPER_SPEED: f64 = 8.0;
const OMEGA: f64 = PI / 18.0;
const TILE_SIZE: f64 = InitTILE_SIZE as f64;

pub struct Physics;

impl Physics {
    // Checks if entities are colliding
    // Params: entityA, entityB
    // Returns: true if entities are colliding, false otherwise
    pub fn check_collision<'a>(entity_a: &mut impl Entity<'a>, entity_b: &mut impl Entity<'a>) -> bool {
        entity_a.hitbox().has_intersection(entity_b.hitbox())
    }

    // Checks if player hasn't landed on their head
    // Params: player, ground position as SDL point, angle of ground
    // Returns: true if player is upright, false otherwise
    pub fn check_player_upright<'a>(player: &Player, angle: f64, ground: Point) -> bool {
        !player.hitbox().contains_point(ground)
            || (player.theta() < OMEGA * 6.0 + angle || player.theta() > 2.0 * PI - OMEGA * 6.0 + angle)
    }

    // Applies terrain forces to a body, i.e. gravity, normal, and friction forces
    // Params: body, angle of ground, ground position as SDL Point, coeff of kinetic
    // friction
    // Returns: none
    pub fn apply_terrain_forces<'a>(
        body: &mut impl Body<'a>,
        angle: f64,
        ground: Point,
        terrain_type: &TerrainType,
        power_up: Option<PowerType>,
    ) {
        // Set Gravity & Friction Strength From TerrainType
        let fric_coeff: f64;
        let mut g: f64 = 1.5;
        //As of now, all conds lead to +accel on flat ground (we could change this)
        match terrain_type {
            TerrainType::Asphalt => {
                //quick accel to max on flat
                fric_coeff = 0.05;
            }
            TerrainType::Grass => {
                //moderate accel to max on flat
                fric_coeff = 0.075;
            }
            TerrainType::Sand => {
                //v slow accel to max on flat & short jumps
                fric_coeff = 0.06; //less friction is more bc higher gravity
                g = 2.0;
            }
            TerrainType::Water => {
                //NOT YET CONFIGURED
                fric_coeff = 0.2;
            }
        }

        // Lower gravity if power is low gravity
        if let Some(PowerType::LowerGravity) = power_up {
            g = g * 2.0 / 3.0;
        }

        // Gravity: mg
        body.apply_force((0.0, -body.mass() * g));

        /*
            Note on angles:
            - Negative angle == uphill
            - Positive angle == downhill
            - sin(-x) is negative
            - cos(-x) is positive
        */

        // If body is on ground, apply normal
        if body.hitbox().contains_point(ground) {
            // Land on ground
            if body.vel_y() < 0.0 || (body.x() as f64 + 0.9 * TILE_SIZE) > ground.y() as f64 {
                body.hard_set_pos((body.x() as f64, ground.y() as f64 - 0.95 * TILE_SIZE));
                body.hard_set_vel((body.vel_x(), 0.0));
                body.align_hitbox_to_pos();
            }

            // Normal: mg, but on an incline
            // (-x, +y) on an uphill
            // (+x, +y) on a downhill
            body.apply_force((body.mass() * g * angle.sin(), body.mass() * g * angle.cos()));

            // If body is on ground AND moving, apply KINETIC FRICTION
            if body.vel_x().abs() + body.vel_y().abs() > 0.0 {
                // Friction: µmg, on an incline, perpendicular to normal
                // (-x, -y) on an uphill
                // (-x, +y) on an downhill
                // make negative if object is moving backwards
                let direction_adjust = body.vel_x().signum();
                body.apply_force((
                    -fric_coeff * body.mass() * g * angle.cos() * direction_adjust,
                    fric_coeff * body.mass() * g * angle.sin() * direction_adjust,
                ));
            }
            // Else if body is on ground and STILL, apply STATIC FRICTION
            // NOTE: This might be unnecessary
            // else {
            //     // (+x, +y) on an uphill
            //     // (-x, +y) on a downhill
            //     body.apply_force((
            //         -angle.signum() * body.mass() * g * angle.cos(),
            //         angle.signum() * body.mass() * g * angle.sin(),
            //     ));
            // }
        }
    }

    // Applies forward motion to player, as if they're propelling themselves
    // Serves to oppose and overcome backwards forces (friction and normal)
    // Params: player, angle of ground, ground position is as SDL Point
    // Returns: None
    pub fn apply_skate_force(player: &mut Player, angle: f64, ground: Point) {
        // Skate force
        let mut skate_force = 1.0 / 8.0 * player.mass();
        if let Some(PowerType::SpeedBoost) = player.power_up() {
            // Speed up with powerup
            skate_force *= 2.0;
        }

        if player.hitbox().contains_point(ground) {
            // (+x, +y) on an uphill
            // (+x, -y) on a downhill
            player.apply_force((skate_force * angle.cos(), -skate_force * angle.sin()));
        }
    }

    // Applies upward spring force using Hooke's law
    // Dependent on player's position: F = kx
    // Params: player, spring object
    // Returns: none
    pub fn apply_bounce<'a>(player: &mut Player, body: &impl Body<'a>) {
        // Spring force constant
        let k = 0.2;

        // Find how far player has depressed the spring
        // let intersection = player.hitbox().intersection(body.hitbox());

        // If the player is really touching the spring, apply the force
        if player.hitbox().has_intersection(body.hitbox()) {
            let displacement = player.hitbox.bottom().y() - body.hitbox().bottom().y();
            // Force is always upwards
            player.apply_force((0.0, k * displacement as f64));
        }
    }

    // Applies upward buoyant force according to Archimedes Principle
    // Dependent on player's area: F = pgV
    // Params: player, surface position as SDL Point
    pub fn apply_buoyancy(player: &mut Player, surface: Point) {
        // Density
        let p = player.mass() / 4.0;

        // Acceleration of gravity
        let mut g: f64 = 1.0;
        if let Some(PowerType::LowerGravity) = player.power_up() {
            // Lower gravity if power is low gravity
            g = 2.0 / 3.0;
        }

        // Calculate player's 2D-volume beneath water
        let submerged_area = player.hitbox().width() as f64
            * (player.hitbox().y() + player.hitbox().height() as i32 - surface.y()) as f64;

        // If the player really is underwater, apply the force
        if submerged_area > 0.0 {
            // Force is always upwards
            player.apply_force((0.0, p * g * submerged_area));
        }
    }
}

/******************************* TRAITS ****************************** */

pub trait Entity<'a> {
    fn texture(&self) -> &Texture<'a>;

    fn x(&self) -> i32 {
        self.hitbox().x()
    }
    fn y(&self) -> i32 {
        self.hitbox().y()
    }
    fn center(&self) -> Point {
        self.hitbox().center()
    }

    fn hitbox(&self) -> PhysRect;
    fn align_hitbox_to_pos(&mut self); // After the pos is set with f64s, this method moves hitbox
                                       // to proper SDL coordinates using i32s

    // Adjusts terrain postion in runner.rs based on camera_adj_x & camera_adj_y
    fn camera_adj(&mut self, x_adj: i32, y_adj: i32);
}

pub trait Body<'a>: Entity<'a> {
    fn mass(&self) -> f64;
    fn rotational_inertia(&self) -> f64 {
        let radius = (self.hitbox().width() as f64) / 2.0;
        self.mass() * radius * radius
    }
    fn update_pos(&mut self, ground: Point, angle: f64, game_over: bool);
    fn hard_set_pos(&mut self, pos: (f64, f64)); // Official method to hardcode position

    fn vel_x(&self) -> f64;
    fn vel_y(&self) -> f64;
    fn update_vel(&mut self, game_over: bool);
    fn hard_set_vel(&mut self, vel: (f64, f64)); // Official method to hardcode velocity

    fn accel_x(&self) -> f64;
    fn accel_y(&self) -> f64;
    fn apply_force(&mut self, force: (f64, f64));
    fn reset_accel(&mut self);

    fn theta(&self) -> f64;
    fn rotate(&mut self);

    fn omega(&self) -> f64;
}

pub trait Collectible<'a>: Entity<'a> {
    fn update_pos(&mut self, x: i32, y: i32);
    fn collect(&mut self);
    fn collected(&self) -> bool;
}

/********************************************************************* */

/****************************** PLAYER ******************************* */

pub struct Player<'a> {
    pub pos: (f64, f64),
    velocity: (f64, f64),
    accel: (f64, f64),
    drawbox: Rect,
    hitbox: PhysRect,

    theta: f64, // angle of rotation, in radians
    omega: f64, // angular speed

    mass: f64,
    texture: &'a Texture<'a>,
    power_up: Option<PowerType>,

    jump_time: SystemTime,
    lock_jump_time: bool,
    jumping: bool,
    flipping: bool,
    second_jump: bool,
}

impl<'a> Player<'a> {
    pub fn new(hitbox: PhysRect, drawbox: Rect, mass: f64, texture: &'a Texture<'a>) -> Player<'a> {
        Player {
            pos: (hitbox.x() as f64, hitbox.y() as f64),
            velocity: (0.0, 0.0),
            accel: (0.0, 0.0),
            hitbox,
            drawbox,

            theta: 0.0,
            omega: 0.0,

            texture,
            mass,
            power_up: None,

            jump_time: SystemTime::now(),
            lock_jump_time: false,
            jumping: true,
            flipping: false,
            second_jump: false,
        }
    }

    pub fn is_jumping(&self) -> bool {
        self.jumping
    }

    pub fn jumpmoment_lock(&self) -> bool {
        self.lock_jump_time
    }

    pub fn is_flipping(&self) -> bool {
        self.flipping
    }

    // Returns specific power-up player has, or None if player hasn't collected a
    // power-up
    pub fn power_up(&self) -> Option<PowerType> {
        self.power_up
    }

    // Setter for power-up
    pub fn set_power_up(&mut self, power_up: Option<PowerType>) {
        self.power_up = power_up;
    }

    // Brings player's rotational velocity to a stop
    pub fn stop_flipping(&mut self) {
        self.flipping = false;
        self.omega = 0.0;
    }

    // Gives player rotational velocity
    pub fn resume_flipping(&mut self) {
        self.flipping = true;
        self.omega = OMEGA;
    }

    pub fn set_jumpmoment(&mut self, time: SystemTime) {
        self.jump_time = time;
        self.lock_jump_time = true;
    }

    pub fn jump_moment(&mut self) -> SystemTime {
        self.jump_time
    }

    // Returns true if a jump was initiated
    pub fn jump(&mut self, ground: Point, duration: Duration) -> bool {
        if self.hitbox().contains_point(ground) {
            // Starting from the position of the ground
            self.hard_set_pos((self.pos.0, ground.y() as f64 - TILE_SIZE));
            self.align_hitbox_to_pos();
            // Apply upward force
            let duration_millis: u128 = duration.as_millis();
            if duration_millis <= Duration::new(0, 100000000).as_millis() {
                self.apply_force((0.0, 60.0));
            } else if duration_millis <= Duration::new(0, 200000000).as_millis() {
                self.apply_force((0.0, 80.0));
            } else {
                self.apply_force((0.0, 100.0));
            }
            //self.apply_force((0.0, 100.0));
            self.jumping = true;
            true
        } else {
            false
        }
    }

    pub fn flip(&mut self) {
        if self.is_flipping() {
            self.rotate();
        }
    }

    // Handles collisions with player and any type of obstacle
    // Params: obstacle to collide with
    // Returns: true if real game-ending collision occurs, false otherwise
    pub fn collide_obstacle(&mut self, obstacle: &mut Obstacle) -> bool {
        let mut shielded = false;
        if let Some(PowerType::Shield) = self.power_up() {
            // Put on shield if applicable
            shielded = true;
        }

        // nearest_side checks for which side of the obstacle had the closest midpoint
        // to any point on the player rectangle
        let collision_side = self.hitbox.nearest_side(obstacle.hitbox());
        if (collision_side == 1 || collision_side == 3) {
            // Response to collision dependent on type of obstacle
            match obstacle.obstacle_type {
                // For statue and chest, elastic collision
                ObstacleType::Statue | ObstacleType::Chest => {
                    if shielded || obstacle.collided() {
                        // If shielded or collision already happened, pretend nothing happened
                        false
                    } else {
                        /********** ELASTIC COLLISION CALCULATION ********* */
                        // https://en.wikipedia.org/wiki/Elastic_collision#One-dimensional_Newtonian
                        // Assumed object has velocity (0,0)
                        // Assumed player has velocity (vx,vy)
                        let angle = ((self.center().y() - obstacle.center().y()) as f64
                            / (self.center().x() - obstacle.center().x()) as f64)
                            .atan();
                        let p_mass = self.mass();
                        let o_mass = obstacle.mass();
                        let p_vx = self.velocity.0;
                        let p_vy = if self.jumping { self.velocity.1 } else { 0.0 };
                        let p_vx_f = 2.0 * (p_mass - o_mass) * (p_vx) / (p_mass + o_mass);
                        let p_vy_f = 2.0 * (p_mass - o_mass) * (p_vy) / (p_mass + o_mass);
                        let o_vx_f = 2.0 * (2.0 * p_mass) * (p_vx) / (p_mass + o_mass);
                        let o_vy_f = 2.0 * (2.0 * p_mass) * (p_vy) / (p_mass + o_mass);

                        // CALCULATE PLAYER AND OBJECT NEW OMEGAS HERE
                        // Torque = r*F * sin(angle)
                        // alpha = Torque/body.rotational_inertia()
                        // For ease of calculation, just set omega = alpha

                        /************************************************** */
                        // Move obstacle
                        obstacle.collided = true;
                        obstacle.hard_set_vel((o_vx_f, o_vy_f));

                        // Move player
                        self.hard_set_vel((p_vx_f, p_vy_f));
                        self.hard_set_pos((obstacle.x() as f64 - 1.05 * TILE_SIZE, self.y() as f64));
                        self.align_hitbox_to_pos();
                        true
                    }
                }
                // For Balloon, do nothing upon SIDE collision
                ObstacleType::Balloon => false,
            }
        } else if self.vel_y() < 0.0 {
            match obstacle.obstacle_type {
                // On top collision with chest, treat the chest as if it's normal ground
                ObstacleType::Chest => {
                    // obstacle.collided = true;
                    self.pos.1 = (obstacle.y() as f64 - 0.95 * (TILE_SIZE as f64));
                    self.align_hitbox_to_pos();
                    self.velocity.1 = 0.0;
                    self.jumping = false;
                    self.lock_jump_time = false;
                    self.apply_force((0.0, self.mass()));
                    self.omega = 0.0;
                    obstacle.collided = true;

                    if self.theta() < OMEGA * 6.0 || self.theta() > 360.0 - OMEGA * 6.0 {
                        self.theta = 0.0;
                        false
                    } else {
                        true
                    }
                }
                // For irregularly shaped statue, player gets hurt and game over
                ObstacleType::Statue => {
                    // bounce for fun
                    Physics::apply_bounce(self, obstacle);
                    true
                }
                // For spring, bounce off with Hooke's law force
                ObstacleType::Balloon => {
                    Physics::apply_bounce(self, obstacle);
                    false
                }
            }
        } else {
            false
        }
    }

    // Collects a coin
    // Params: coin to collect
    // Returns: true if coin has been collected, false otherwise (e.g. if it's been
    // collected already)
    pub fn collide_coin(&mut self, coin: &mut Coin) -> bool {
        if !coin.collected() {
            coin.collect();
            true
        } else {
            false
        }
    }

    // Receives new power-up
    // Params: power to use
    // Returns:
    pub fn collide_power(&mut self, power: &mut Power) -> bool {
        if !power.collected() {
            self.set_power_up(Some(power.power_type()));
            power.collect();
            true
        } else {
            false
        }
    }
}

impl<'a> Entity<'a> for Player<'a> {
    fn texture(&self) -> &Texture<'a> {
        self.texture
    }

    fn hitbox(&self) -> PhysRect {
        self.hitbox
    }

    fn align_hitbox_to_pos(&mut self) {
        self.hitbox.set_x(self.pos.0 as i32);
        self.hitbox.set_y(self.pos.1 as i32);
    }

    // Adjusts terrain postion in runner.rs based on camera_adj_x & camera_adj_y
    fn camera_adj(&mut self, x_adj: i32, y_adj: i32) {
        self.pos.0 += (x_adj as f64);
        self.pos.1 += (y_adj as f64);

        self.align_hitbox_to_pos();
    }
}

impl<'a> Body<'a> for Player<'a> {
    fn mass(&self) -> f64 {
        self.mass
    }

    fn update_pos(&mut self, ground: Point, angle: f64, game_over: bool) {
        if self.hitbox.contains_point(ground) {
            self.theta = angle;
        }

        /*
        // TEMPORARY: Player's x position is fixed until camera freezes on game ending
        // Will change when camera follows player
        if game_over {
            self.pos.0 += self.vel_x();
        }
        */
        self.pos.1 -= self.vel_y();

        // Match the angle of the ground if on ground
        if self.hitbox.contains_point(ground) && !game_over {
            self.theta = angle;
            if self.jumping {
                self.jumping = false;
                self.lock_jump_time = false;
            }
        }

        self.align_hitbox_to_pos();
    }

    fn hard_set_pos(&mut self, pos: (f64, f64)) {
        self.pos.0 = pos.0;
        self.pos.1 = pos.1;
    }

    fn vel_x(&self) -> f64 {
        self.velocity.0
    }

    fn vel_y(&self) -> f64 {
        self.velocity.1
    }

    fn update_vel(&mut self, game_over: bool) {
        if game_over {
            self.velocity.0 = (self.velocity.0 + self.accel.0).clamp(LOWER_SPEED, UPPER_SPEED);
        } else {
            self.velocity.0 = (self.velocity.0 + self.accel.0).clamp(1.0, UPPER_SPEED);
        }

        self.velocity.1 = (self.velocity.1 + self.accel.1).clamp(3.0 * LOWER_SPEED, 5.0 * UPPER_SPEED);
    }

    fn hard_set_vel(&mut self, vel: (f64, f64)) {
        self.velocity.0 = vel.0;
        self.velocity.1 = vel.1;
    }

    fn accel_x(&self) -> f64 {
        self.accel.0
    }

    fn accel_y(&self) -> f64 {
        self.accel.1
    }

    fn apply_force(&mut self, force: (f64, f64)) {
        self.accel.0 += force.0 / self.mass();
        self.accel.1 += force.1 / self.mass();
    }

    fn reset_accel(&mut self) {
        self.accel = (0.0, 0.0);
    }

    fn theta(&self) -> f64 {
        self.theta
    }

    fn rotate(&mut self) {
        self.theta = (self.theta - self.omega() + 2.0 * PI) % (2.0 * PI);
    }

    fn omega(&self) -> f64 {
        self.omega
    }
}

/********************************************************************* */

/*************************** OBSTACLE ******************************** */

pub struct Obstacle<'a> {
    pub pos: (f64, f64),
    velocity: (f64, f64),
    accel: (f64, f64),
    hitbox: PhysRect,

    mass: f64,
    texture: &'a Texture<'a>,
    obstacle_type: ObstacleType,

    theta: f64,
    omega: f64,

    pub collided: bool,
    pub spawned: bool,
    pub delete_me: bool,
}

impl<'a> Obstacle<'a> {
    pub fn new(hitbox: PhysRect, mass: f64, texture: &'a Texture<'a>, obstacle_type: ObstacleType) -> Obstacle<'a> {
        Obstacle {
            pos: (hitbox.x() as f64, hitbox.y() as f64),
            velocity: (0.0, 0.0),
            accel: (0.0, 0.0),
            hitbox,

            mass,
            texture,
            obstacle_type,

            theta: 0.0,
            omega: 0.0,

            collided: false,
            spawned: false,
            delete_me: false,
        }
    }

    pub fn obstacle_type(&self) -> ObstacleType {
        self.obstacle_type
    }

    pub fn collided(&self) -> bool {
        self.collided
    }

    // Shifts objects left with the terrain in runner.rs
    pub fn travel_update(&mut self, travel_adj: i32) {
        self.pos.0 -= (travel_adj as f64);
    }
}

impl<'a> Entity<'a> for Obstacle<'a> {
    fn texture(&self) -> &Texture<'a> {
        self.texture
    }

    fn hitbox(&self) -> PhysRect {
        self.hitbox
    }

    fn align_hitbox_to_pos(&mut self) {
        self.hitbox.set_x(self.pos.0 as i32);
        self.hitbox.set_y(self.pos.1 as i32);
    }

    // Adjusts terrain postion in runner.rs based on camera_adj_x & camera_adj_y
    fn camera_adj(&mut self, x_adj: i32, y_adj: i32) {
        self.pos.0 += (x_adj as f64);
        self.pos.1 += (y_adj as f64);

        self.align_hitbox_to_pos();
    }
}

impl<'a> Body<'a> for Obstacle<'a> {
    fn mass(&self) -> f64 {
        self.mass
    }

    fn update_pos(&mut self, ground: Point, angle: f64, game_over: bool) {
        if self.hitbox.contains_point(ground) && !game_over {
            self.theta = angle;
        }

        self.pos.0 += self.vel_x();
        self.pos.1 -= self.vel_y();
        self.align_hitbox_to_pos();
    }

    fn hard_set_pos(&mut self, pos: (f64, f64)) {
        self.pos.0 = pos.0;
        self.pos.1 = pos.1;
    }

    fn vel_x(&self) -> f64 {
        self.velocity.0
    }

    fn vel_y(&self) -> f64 {
        self.velocity.1
    }

    fn update_vel(&mut self, game_over: bool) {
        self.velocity.0 = (self.velocity.0 + self.accel.0).clamp(-20.0, 20.0);
        self.velocity.1 = (self.velocity.1 + self.accel.1).clamp(-20.0, 20.0);
    }

    fn hard_set_vel(&mut self, vel: (f64, f64)) {
        self.velocity.0 = vel.0;
        self.velocity.1 = vel.1;
    }

    fn accel_x(&self) -> f64 {
        self.accel.0
    }

    fn accel_y(&self) -> f64 {
        self.accel.1
    }

    fn apply_force(&mut self, force: (f64, f64)) {
        self.accel.0 += force.0 / self.mass();
        self.accel.1 += force.1 / self.mass();
    }

    fn reset_accel(&mut self) {
        self.accel = (0.0, 0.0);
    }

    fn theta(&self) -> f64 {
        self.theta
    }

    fn rotate(&mut self) {
        self.theta = (self.theta - self.omega() + 2.0 * PI) % (2.0 * PI);
    }

    fn omega(&self) -> f64 {
        self.omega
    }
}

/********************************************************************* */

/**************************** COIN *********************************** */

pub struct Coin<'a> {
    pub pos: (i32, i32),
    hitbox: PhysRect,
    texture: &'a Texture<'a>,
    value: i32,
    collected: bool,
}

impl<'a> Coin<'a> {
    pub fn new(hitbox: PhysRect, texture: &'a Texture<'a>, value: i32) -> Coin<'a> {
        Coin {
            pos: (hitbox.x(), hitbox.y()),
            texture,
            hitbox,
            value,
            collected: false,
        }
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    // Shifts objects left with the terrain in runner.rs
    pub fn travel_update(&mut self, travel_adj: i32) {
        self.pos.0 -= travel_adj;
    }
}

impl<'a> Entity<'a> for Coin<'a> {
    fn texture(&self) -> &Texture<'a> {
        self.texture
    }

    fn hitbox(&self) -> PhysRect {
        self.hitbox
    }

    fn align_hitbox_to_pos(&mut self) {
        self.hitbox.set_x(self.pos.0);
        self.hitbox.set_y(self.pos.1);
    }

    // Adjusts terrain postion in runner.rs based on camera_adj_x & camera_adj_y
    fn camera_adj(&mut self, x_adj: i32, y_adj: i32) {
        self.pos.0 += x_adj;
        self.pos.1 += y_adj;

        self.align_hitbox_to_pos();
    }
}

impl<'a> Collectible<'a> for Coin<'a> {
    fn update_pos(&mut self, x: i32, y: i32) {
        self.pos.0 = x;
        self.pos.1 = y;
    }

    fn collect(&mut self) {
        self.collected = true;
    }

    fn collected(&self) -> bool {
        self.collected
    }
}

/********************************************************************* */

/*************************** POWER *********************************** */

pub struct Power<'a> {
    pub pos: (i32, i32),
    hitbox: PhysRect,
    texture: &'a Texture<'a>,
    power_type: PowerType,
    collected: bool,
}

impl<'a> Power<'a> {
    pub fn new(hitbox: PhysRect, texture: &'a Texture<'a>, power_type: PowerType) -> Power<'a> {
        Power {
            pos: (hitbox.x(), hitbox.y()),
            hitbox,
            texture,
            collected: false,
            power_type,
        }
    }

    pub fn power_type(&self) -> PowerType {
        self.power_type
    }

    // Shifts objects left with the terrain in runner.rs
    pub fn travel_update(&mut self, travel_adj: i32) {
        self.pos.0 -= travel_adj;
    }
}

impl<'a> Entity<'a> for Power<'a> {
    fn texture(&self) -> &Texture<'a> {
        self.texture
    }

    fn hitbox(&self) -> PhysRect {
        self.hitbox
    }

    fn align_hitbox_to_pos(&mut self) {
        self.hitbox.set_x(self.pos.0 as i32);
        self.hitbox.set_y(self.pos.1 as i32);
    }

    // Adjusts terrain postion in runner.rs based on camera_adj_x & camera_adj_y
    fn camera_adj(&mut self, x_adj: i32, y_adj: i32) {
        self.pos.0 += x_adj;
        self.pos.1 += y_adj;

        self.align_hitbox_to_pos();
    }
}

impl<'a> Collectible<'a> for Power<'a> {
    fn update_pos(&mut self, x: i32, y: i32) {
        self.pos.0 = x;
        self.pos.1 = y;
    }

    fn collect(&mut self) {
        self.collected = true;
    }

    fn collected(&self) -> bool {
        self.collected
    }
}

/******************************ROTATING
 * HITBOX******************************* */

/// The maximal integer value that can be used for rectangles.
///
/// This value is smaller than strictly needed, but is useful in ensuring that
/// rect sizes will never have to be truncated when clamping.
pub fn max_int_value() -> u32 {
    i32::max_value() as u32 / 2
}

/// The minimal integer value that can be used for rectangle positions
/// and points.
///
/// This value is needed, because otherwise the width of a rectangle created
/// from a point would be able to exceed the maximum width.
pub fn min_int_value() -> i32 {
    i32::min_value() / 2
}

fn clamp_size(val: u32) -> u32 {
    if val == 0 {
        1
    } else if val > max_int_value() {
        max_int_value()
    } else {
        val
    }
}

fn clamp_position(val: i32) -> i32 {
    if val > max_int_value() as i32 {
        max_int_value() as i32
    } else if val < min_int_value() {
        min_int_value()
    } else {
        val
    }
}

// converts angle to an equivalent value between 0 and 2π
fn clamp_angle(val: f64) -> f64 {
    val % (2.0 * PI)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    theta: f64,
    coords: [Point; 4],
}

impl PhysRect {
    // rectangle with no rotation applied
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> PhysRect {
        let x = clamp_position(x);
        let y = clamp_position(y);
        let w = clamp_size(width) as i32;
        let h = clamp_size(height) as i32;
        PhysRect {
            x,
            y,
            w,
            h,
            theta: 0.0,
            coords: [
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
            ],
        }
    }

    pub fn from_center<P>(center: P, width: u32, height: u32) -> PhysRect
    where
        P: Into<Point>,
    {
        let w = clamp_size(width) as i32;
        let h = clamp_size(height) as i32;
        let mut rect = PhysRect {
            x: 0,
            y: 0,
            w,
            h,
            theta: 0.0,
            coords: [Point::new(0, 0), Point::new(w, 0), Point::new(w, h), Point::new(0, h)],
        };
        rect.center_on(center.into());
        rect
    }

    pub fn as_rect(&self) -> Rect {
        Rect::from_center(self.center(), self.width(), self.height())
    }

    /// The horizontal position of the original top left corner of the
    /// rectangle.
    pub fn x(&self) -> i32 {
        self.x
    }

    /// The vertical position of the original top left corner of this rectangle.
    pub fn y(&self) -> i32 {
        self.y
    }

    /// The four corners of the rectangle in clockwise order starting with the
    /// original top left
    pub fn coords(&self) -> [Point; 4] {
        self.coords
    }

    /// The width of this rectangle
    pub fn width(&self) -> u32 {
        self.w as u32
    }

    /// The height of this rectangle
    pub fn height(&self) -> u32 {
        self.h as u32
    }

    /// The rotation angle of this rectangle
    pub fn angle(&self) -> f64 {
        self.theta
    }

    /// Sets the vertical position of this rectangle to the given value,
    /// clamped to be less than or equal to i32::max_value() / 2.
    /// Position is based on the original upper right corner of the rectangle.
    pub fn set_x(&mut self, x: i32) {
        let d = x - self.x();
        self.x = clamp_position(x);
        for i in 0..self.coords.len() {
            self.coords[i] = self.coords[i].offset(d, 0);
        }
    }

    /// Sets the vertical position of this rectangle to the given value,
    /// clamped to be less than or equal to i32::max_value() / 2.
    /// Position is based on the original upper right corner of the rectangle.
    pub fn set_y(&mut self, y: i32) {
        let d = y - self.y();
        self.y = clamp_position(y);
        for i in 0..self.coords.len() {
            self.coords[i] = self.coords[i].offset(0, d);
        }
    }

    pub fn set_angle(&mut self, theta: f64) {
        let d = theta - self.angle();
        self.rotate(d);
    }

    /// Sets the height of this rectangle to the given value,
    /// clamped to be less than or equal to i32::max_value() / 2.
    pub fn set_height(&mut self, height: u32) {
        self.h = clamp_size(height) as i32;
    }

    /// The rectangle's current leftmost point
    pub fn left(&self) -> Point {
        let mut left = self.coords[0];
        for p in self.coords {
            if p.x() <= left.x() {
                left = p;
            }
        }
        left
    }

    /// The rectangle's current rightmost point
    pub fn right(&self) -> Point {
        let mut right = self.coords[0];
        for p in self.coords {
            if p.x() >= right.x() {
                right = p;
            }
        }
        right
    }

    /// The rectangle's current topmost point
    pub fn top(&self) -> Point {
        let mut top = self.coords[0];
        for p in self.coords {
            if p.y() <= top.y() {
                top = p;
            }
        }
        top
    }

    /// The rectangle's current bottom-most point
    pub fn bottom(&self) -> Point {
        let mut bottom = self.coords[0];
        for p in self.coords {
            if p.y() <= bottom.y() {
                bottom = p;
            }
        }
        bottom
    }

    /// The rectangle's center point
    pub fn center(&self) -> Point {
        let x = (self.coords[0].x() + self.coords[2].x()) / 2;
        let y = (self.coords[0].y() + self.coords[2].y()) / 2;
        Point::new(x, y)
    }

    // Centers the rectangle on point P
    pub fn center_on<P>(&mut self, point: P)
    where
        P: Into<(i32, i32)>,
    {
        let (x, y) = point.into();
        let d_x = clamp_position(x) - self.center().x();
        let d_y = clamp_position(y) - self.center().y();
        for p in self.coords {
            p.offset(d_x, d_y);
        }
        self.x = self.coords[0].x();
        self.y = self.coords[0].y();
    }

    /// Move this rect and clamp the positions to prevent over/underflow.
    /// This also clamps the size to prevent overflow.
    pub fn offset(&mut self, x: i32, y: i32) {
        let old_x = self.x;
        let old_y = self.y;
        match self.x.checked_add(x) {
            Some(val) => self.x = clamp_position(val),
            None => {
                if x >= 0 {
                    self.x = max_int_value() as i32;
                } else {
                    self.x = i32::min_value();
                }
            }
        }
        match self.y.checked_add(y) {
            Some(val) => self.y = clamp_position(val),
            None => {
                if y >= 0 {
                    self.y = max_int_value() as i32;
                } else {
                    self.y = i32::min_value();
                }
            }
        }
        let d_x = old_x - self.x;
        let d_y = old_x - self.y;
        for i in 0..self.coords.len() {
            self.coords[i] = self.coords[i].offset(d_x, d_y);
        }
    }

    /// Moves this rect to the given position after clamping the values.
    pub fn reposition<P>(&mut self, point: P)
    where
        P: Into<(i32, i32)>,
    {
        let (x, y) = point.into();
        let old_x = self.x();
        let old_y = self.y();
        self.x = clamp_position(x);
        self.y = clamp_position(y);
        let d_x = old_x - self.x();
        let d_y = old_x - self.y();
        for i in 0..self.coords.len() {
            self.coords[i] = self.coords[i].offset(d_x, d_y);
        }
    }

    /// Resizes this rect to the given size after clamping the values
    pub fn resize(&mut self, width: u32, height: u32) {
        let d_w = (width - self.width()) as f64;
        let d_h = (height - self.height()) as f64;
        let dist = (d_w.powi(2) + d_h.powi(2)).sqrt();
        self.coords[1] = self.coords[1].offset((d_w * self.angle().cos()) as i32, (d_w * self.angle().sin()) as i32);
        self.coords[2] = self.coords[2].offset((dist * self.angle().cos()) as i32, (dist * self.angle().sin()) as i32);
        self.coords[3] = self.coords[3].offset((d_h * self.angle().cos()) as i32, (d_w * self.angle().sin()) as i32);
        self.w = clamp_size(width) as i32;
        self.h = clamp_size(height) as i32;
    }

    pub fn rotate(&mut self, theta: f64) {
        let c = self.center();
        for i in 0..self.coords.len() {
            let x = theta.cos() * (self.coords[i].x() - c.x()) as f64
                - theta.sin() * (self.coords[i].y() - c.y()) as f64
                + c.x() as f64;
            let y = theta.sin() * (self.coords[i].x() - c.x()) as f64
                + theta.cos() * (self.coords[i].y() - c.y()) as f64
                + c.y() as f64;
            self.coords[i] = Point::new(x as i32, y as i32)
        }
        self.theta = theta;
        self.x = self.coords[0].x();
        self.y = self.coords[0].y();
    }

    /// Checks whether this rect contains a given point
    pub fn contains_point<P>(&self, point: P) -> bool
    where
        P: Into<(i32, i32)>,
    {
        let (x, y) = point.into();
        let mut c = false;
        let mut j = 3;
        for i in 0..self.coords.len() {
            if (((self.coords[i].y() > y) != (self.coords[j].y() > y))
                && (x
                    < (self.coords[j].x() - self.coords[i].x()) * (y - self.coords[i].y())
                        / (self.coords[j].y() - self.coords[i].y())
                        + self.coords[i].x()))
            {
                c = !c;
            }
            j = i;
        }
        c
    }

    /// Checks whether this rect intersects a given rect
    pub fn has_intersection(&self, other: PhysRect) -> bool {
        for i in 0..other.coords.len() {
            if self.contains_point(other.coords[i]) {
                return true;
            }
        }
        for i in 0..self.coords.len() {
            if other.contains_point(self.coords[i]) {
                return true;
            }
        }
        false
    }

    /// Returns an integer corresponding to the side of this rect that the given
    /// rect's points are closest to 0, 1, 2, and 3 correspond to top,
    /// right, bottom, and left respectively Mainly used for collision logic
    pub fn nearest_side(&self, other: PhysRect) -> i32 {
        // store and index the midpoints of the given rectangle
        let mut mids = Vec::new();
        let mut j = 3;
        for i in 0..other.coords.len() {
            let p = Point::new(
                (other.coords[i].x() + other.coords[j].x()) / 2,
                (other.coords[i].y() + other.coords[j].y) / 2,
            );
            mids.push(p);
            j = i;
        }
        // find the side of the given rectangle whose midpoint is closest to a point in
        // this rectangle
        let mut min_dist = f64::MAX;
        let mut min_side = 0;
        for i in 0..self.coords.len() {
            for p in &mids {
                let dist = (((self.coords[i].x() - p.x()) as f64).powi(2)
                    + ((self.coords[i].y() - p.y()) as f64).powi(2))
                .sqrt();
                if dist <= min_dist {
                    min_dist = dist;
                    min_side = i as i32;
                }
            }
        }
        min_side
    }
}

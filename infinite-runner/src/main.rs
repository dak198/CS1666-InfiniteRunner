extern crate sdl_rust;

use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::event::Event;

use sdl_rust::SDLCore;
use sdl_rust::Demo;

const TITLE: &str = "SDL04 Event Handling";
const CAM_W: u32 = 640;
const CAM_H: u32 = 480;
// No timeout needed!

pub struct CREDITS {
	core: SDLCore,
}

impl Demo for CREDITS {
	fn init() -> Result<Self, String> {
		let core = SDLCore::init(TITLE, true, CAM_W, CAM_H)?;
		Ok(CREDITS{ core })
	}

	fn run(&mut self) -> Result<(), String> {
        let ttf_cxt = sdl2::ttf::init().map_err(|e| e.to_string())?;

        let texture_creator = self.core.wincan.texture_creator();

        let mut font = ttf_cxt.load_font("./assets/Debrosee-ALPnL.ttf", 128)?;

        let surface = font
            .render("Example text")
            .blended(Color::BLUE)
            .map_err(|e| e.to_string())?;
        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .map_err(|e| e.to_string())?;
        let center = Point::new(320, 240);
        let w = 400;
        let h = 200;
        
		'gameloop: loop {
			for event in self.core.event_pump.poll_iter() {
				match event {
					Event::Quit{..} => break 'gameloop,
					_ => {},
				}
			}

			// Draw (or re-draw) SDL03 demo
			self.draw_demo(&texture, &center, &w, &h)?;
		}

		// Out of game loop, return Ok
		Ok(())
	}
}

impl CREDITS {
	// Code from SDL03 repeated...
	fn draw_demo(&mut self,
            texture: &sdl2::render::Texture,
            location: &sdl2::rect::Point,
            width: &u32,
            height: &u32
        ) -> Result<(), String> {
		self.core.wincan.clear();
		self.core.wincan.set_draw_color(Color::GREEN);
        let target = Rect::from_center(*location, *width, *height);
        self.core.wincan.copy(texture, None, Some(target))?;
		self.core.wincan.present();

		Ok(())
	}
}

fn main() {
	sdl_rust::runner(TITLE, CREDITS::init);
}
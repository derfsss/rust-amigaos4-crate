//! sdl3-demo: the safe `sdl3` crate on AmigaOS 4 (the cross-compiler
//! image's clib4 libSDL3.a). A window with a renderer, a streaming ARGB
//! texture drawn alpha-blended and colour-tinted, the arrow keys and the
//! mouse moving it, gamepads listed, a 440 Hz tone for half a second, and
//! a report of what happened. Quits on Escape, the close gadget or after
//! 12 seconds.

use std::time::{Duration, Instant};

use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream};
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::mouse::MouseButton;
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{BlendMode, FRect};

struct Tone {
    phase: f32,
    step: f32,
    buf: Vec<f32>,
}

impl AudioCallback<f32> for Tone {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        self.buf.resize(requested.max(0) as usize, 0.0);
        for s in self.buf.iter_mut() {
            *s = if self.phase < 0.5 { 0.2 } else { -0.2 };
            self.phase = (self.phase + self.step) % 1.0;
        }
        let _ = stream.put_data_f32(&self.buf);
    }
}

fn cstr(p: *const core::ffi::c_char) -> String {
    if p.is_null() {
        return "-".into();
    }
    // SAFETY: SDL returns NUL-terminated strings it owns.
    unsafe { core::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let audio = sdl.audio()?;
    let gamepads = sdl.gamepad()?;
    let v = sdl3::version::version();
    println!("SDL {}.{}.{}, video driver {}", v.major, v.minor, v.patch, video.current_video_driver());

    let window = video.window("sdl3-demo", 640, 400).position_centered().build()?;
    let mut canvas = window.into_canvas();
    // SAFETY: the canvas's renderer pointer is valid while the canvas lives.
    println!("renderer {}", cstr(unsafe { sdl3::sys::render::SDL_GetRendererName(canvas.raw()) }));

    // A 32x32 ARGB checker with transparent and half-transparent squares.
    let creator = canvas.texture_creator();
    let mut tex = creator.create_texture_streaming(PixelFormat::ARGB8888, 32, 32)?;
    tex.with_lock(None, |buf: &mut [u8], pitch: usize| {
        for y in 0..32 {
            for x in 0..32 {
                let a: u32 = if ((x / 8) + (y / 8)) % 2 == 0 { 0xFF } else { 0x80 };
                let px = (a << 24) | 0x00FF_FFFF;
                buf[y * pitch + x * 4..y * pitch + x * 4 + 4].copy_from_slice(&px.to_ne_bytes());
            }
        }
    })?;
    tex.set_blend_mode(BlendMode::Blend);

    let pads = gamepads.gamepads().map(|p| p.len()).unwrap_or(0);
    println!("{} gamepads", pads);

    let spec = AudioSpec { freq: Some(22050), channels: Some(1), format: Some(AudioFormat::f32_sys()) };
    let tone = audio.open_playback_stream(&spec, Tone { phase: 0.0, step: 440.0 / 22050.0, buf: Vec::new() });
    let mut tone = match tone {
        Ok(t) => {
            t.resume()?;
            // SAFETY: a plain query.
            println!("audio driver {}, tone playing", cstr(unsafe { sdl3::sys::audio::SDL_GetCurrentAudioDriver() }));
            Some(t)
        }
        Err(e) => {
            println!("audio: {}", e);
            None
        }
    };

    let mut pump = sdl.event_pump()?;
    let (mut x, mut y) = (300.0f32, 180.0f32);
    let (mut keys, mut clicks, mut moves, mut frames) = (0u32, 0u32, 0u32, 0u32);
    let start = Instant::now();
    let mut quit = "time";
    'run: while start.elapsed() < Duration::from_secs(12) {
        for e in pump.poll_iter() {
            match e {
                Event::Quit { .. } => {
                    quit = "close";
                    break 'run;
                }
                Event::KeyDown { keycode: Some(k), .. } => {
                    keys += 1;
                    println!("key {}", k.name());
                    match k {
                        Keycode::Escape => {
                            quit = "Escape";
                            break 'run;
                        }
                        Keycode::Left => x -= 16.0,
                        Keycode::Right => x += 16.0,
                        Keycode::Up => y -= 16.0,
                        Keycode::Down => y += 16.0,
                        _ => {}
                    }
                }
                Event::MouseButtonDown { mouse_btn, x: mx, y: my, .. } => {
                    clicks += 1;
                    println!("click {:?} at {:.0},{:.0}", mouse_btn, mx, my);
                    if mouse_btn == MouseButton::Left {
                        x = mx - 32.0;
                        y = my - 32.0;
                    }
                }
                Event::MouseMotion { .. } => moves += 1,
                _ => {}
            }
        }
        if tone.is_some() && start.elapsed() > Duration::from_millis(500) {
            tone = None;
        }
        let t = start.elapsed().as_millis() as u32;
        canvas.set_draw_color(Color::RGB(20, 20, 60));
        canvas.clear();
        // The tint cycles through red, green and blue each second.
        let phase = (t / 333) % 3;
        tex.set_color_mod(if phase == 0 { 255 } else { 80 }, if phase == 1 { 255 } else { 80 }, if phase == 2 { 255 } else { 80 });
        canvas.copy(&tex, None, Some(FRect::new(x, y, 64.0, 64.0)))?;
        canvas.present();
        frames += 1;
        std::thread::sleep(Duration::from_millis(15));
    }
    let secs = start.elapsed().as_secs_f32();
    println!(
        "quit by {}: {} frames in {:.1} s ({:.1} fps), {} keys, {} clicks, {} mouse moves",
        quit,
        frames,
        secs,
        frames as f32 / secs,
        keys,
        clicks,
        moves
    );
    Ok(())
}

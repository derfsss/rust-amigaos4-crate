//! sdl3-hello: SDL3 from Rust on AmigaOS 4 (the clib4 libSDL3.a of the
//! cross-compiler image, through the sdl3-sys bindings). Opens a window
//! and a renderer, draws a streamed texture moving for about a second,
//! polls events, and quits. Prints what it found.

use core::ffi::{c_int, CStr};
use core::ptr::null_mut;

use sdl3_sys::everything::*;

fn error() -> String {
    // SAFETY: SDL_GetError returns a NUL-terminated string SDL owns.
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

fn main() {
    // SAFETY: plain SDL calls in the order SDL documents; every object is
    // destroyed before SDL_Quit.
    unsafe {
        if !SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO | SDL_INIT_GAMEPAD) {
            println!("SDL_Init: {}", error());
            std::process::exit(20);
        }
        let v = SDL_GetVersion();
        let driver = SDL_GetCurrentVideoDriver();
        let driver = if driver.is_null() { "-".into() } else { CStr::from_ptr(driver).to_string_lossy() };
        println!("SDL {}.{}.{}, video driver {}", v / 1_000_000, (v / 1000) % 1000, v % 1000, driver);

        let mut w = null_mut();
        let mut r = null_mut();
        if !SDL_CreateWindowAndRenderer(c"sdl3-hello".as_ptr(), 320, 200, SDL_WindowFlags(0), &mut w, &mut r) {
            println!("window: {}", error());
            SDL_Quit();
            std::process::exit(20);
        }
        println!("renderer {}", CStr::from_ptr(SDL_GetRendererName(r)).to_string_lossy());

        // A 16x16 ARGB texture: a checker in two colours, alpha-blended.
        let t = SDL_CreateTexture(r, SDL_PIXELFORMAT_ARGB8888, SDL_TEXTUREACCESS_STATIC, 16, 16);
        let px: Vec<u32> = (0..256u32).map(|i| if ((i / 16) ^ (i % 16)) & 4 != 0 { 0xC0FF_8000 } else { 0xC000_80FF }).collect();
        SDL_UpdateTexture(t, null_mut(), px.as_ptr().cast(), 16 * 4);
        SDL_SetTextureBlendMode(t, SDL_BLENDMODE_BLEND);

        let mut events = 0;
        for i in 0..60 {
            let mut e = core::mem::zeroed::<SDL_Event>();
            while SDL_PollEvent(&mut e) {
                events += 1;
            }
            SDL_SetRenderDrawColor(r, 0, 0, (i * 4) as u8, 255);
            SDL_RenderClear(r);
            let dst = SDL_FRect { x: (i * 4) as f32, y: 50.0, w: 64.0, h: 64.0 };
            SDL_RenderTexture(r, t, null_mut(), &dst);
            SDL_RenderPresent(r);
            SDL_Delay(16);
        }
        let mut pads: c_int = 0;
        let ids = SDL_GetGamepads(&mut pads);
        SDL_free(ids.cast());
        println!("60 frames, {} events, {} gamepads", events, pads);

        SDL_DestroyTexture(t);
        SDL_DestroyRenderer(r);
        SDL_DestroyWindow(w);
        SDL_Quit();
        println!("done");
    }
}

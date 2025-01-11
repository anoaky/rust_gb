use rust_gb::{run_cpu, timer, GbInput};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::EventPump;
use std::process::exit;
use std::time::{Duration, Instant};

const PAL_BW: [[u8; 3]; 4] = [[155, 188, 15], [139, 172, 15], [48, 98, 48], [15, 56, 15]];

fn draw_frame(canvas: &mut WindowCanvas, frame: Vec<u8>) {
    for y in 0..144usize {
        for x in 0..160usize {
            let px: u8 = frame[y * 160 + x];
            let pal = PAL_BW[px as usize];
            let colour: Color = Color::RGB(pal[0], pal[1], pal[2]);
            canvas.set_draw_color(colour);
            canvas
                .fill_rect(Rect::new(x as i32 * 5, y as i32 * 5, 5, 5))
                .unwrap();
        }
    }
}

fn main() {
    let sdl = sdl2::init().unwrap();
    let video_subsys = sdl.video().unwrap();
    let window = video_subsys
        .window("Rustyboy", 160 * 5, 144 * 5)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_draw_color(Color::RGB(0x9A, 0x9E, 0x3F));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl.event_pump().unwrap();
    let args: Vec<String> = std::env::args().collect();

    let (gbin_tx, gbout_rx) = run_cpu(&args[1]);
    'game: loop {
        let timer = Instant::now();
        for event in event_pump.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'game,
                _ => (),
            }
        }
        gbin_tx.send(GbInput {}).unwrap();
        match gbout_rx.recv() {
            Ok(gbout) => {
                canvas.clear();
                draw_frame(&mut canvas, gbout);
                canvas.present();
            }
            Err(_) => break 'game,
        }
        let ft: u64 = timer.elapsed().as_nanos() as u64;
        let fps = 1_000_000_000f64 / ft as f64;
        let frame_time: f64 = (ft as f64) / 1_000_000f64;
        canvas
            .window_mut()
            .set_title(format!("Rustyboy {:.2} fps | {:.2} ms", fps, frame_time).as_str())
            .unwrap();
    }
}

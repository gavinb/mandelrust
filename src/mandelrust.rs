//============================================================================
//
// A simple Mandelbrot image generator in Rust
//
// Main entry point for GUI
//
// Copyright (c) 2014 Gavin Baker <gavinb@antonym.org>
// Published under the MIT license
//
//============================================================================

#[macro_use]
extern crate glium;
extern crate glutin;

//----------------------------------------------------------------------------

fn main() {

    use glium::{DisplayBuild,Surface};

    let window = glium::glutin::WindowBuilder::new()
        .with_dimensions(512, 512)
        .with_title("MandelRust".to_string())
        .build_glium()
        .unwrap();

    loop {
        let mut target = window.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);
        target.finish().unwrap();

        for ev in window.poll_events() {
            match ev {
                glium::glutin::Event::Closed => return,
                _ => ()
            }
        }
    }
}

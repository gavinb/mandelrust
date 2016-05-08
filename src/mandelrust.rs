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

use gui::WindowController;

mod gui;
mod engine;
mod protocol;
mod shaders;

//----------------------------------------------------------------------------

fn main() {

    use glium::{DisplayBuild,Surface};

    let mut display = glium::glutin::WindowBuilder::new()
        .with_dimensions(512, 512)
        .with_title("MandelRust".to_string())
        .build_glium()
        .unwrap();

    let mut win_ctrl = WindowController::new(&mut display);

    println!("Running...");
    win_ctrl.run();
    println!("Done");
}

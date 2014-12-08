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

#![feature(globs)]
#![feature(phase)]

extern crate glfw;
extern crate gleam;

use gui::WindowController;

mod gui;
mod protocol;
mod engine;

//----------------------------------------------------------------------------

fn main() {

    println!("GLFW version: {}", glfw::get_version_string());

    let (glfw, errors) = glfw::init().unwrap();
    glfw::fail_on_errors(&errors);

    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 2));
    glfw.window_hint(glfw::WindowHint::OpenglForwardCompat(true));
    glfw.window_hint(glfw::WindowHint::OpenglProfile(glfw::OpenGlProfileHint::Core));
    glfw.window_hint(glfw::WindowHint::Resizable(false));

    let (window, events) = glfw.create_window(512, 512,
                                              "MandelRust",
                                              glfw::WindowMode::Windowed)
        .expect({glfw::fail_on_errors(&errors);
                 "Failed to create GLFW window."});

    glfw.make_context_current(Some(&window));
    window.set_key_polling(true);

    let mut win_ctrl = WindowController::new(&window);

    win_ctrl.start_engine();

    while !window.should_close() {
        glfw.poll_events();
        for event in glfw::flush_messages(&events) {
            win_ctrl.handle_window_event(&window, event);
        }
        win_ctrl.maybe_update_display();
        win_ctrl.draw();
    }
}

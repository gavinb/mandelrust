//============================================================================
//
// A simple Mandelbrot image generator in Rust
//
// Glutin/Glium GUI Front-end
//
// Copyright (c) 2014 Gavin Baker <gavinb@antonym.org>
// Published under the MIT license
//
//============================================================================

#![allow(unused_must_use)]

use std::sync::mpsc::{channel, Sender, Receiver};
use std::vec::Vec;
use std::thread;
use std::io::Cursor;

use glium;
use glium::{Display, Surface};
use glium::glutin::{Event,VirtualKeyCode,ElementState};
use glium::backend::glutin_backend::GlutinFacade;
use image;

use engine::MandelEngine;
use protocol::{RenderType, EngineCommand, EngineStatus, PREVIEW_WIDTH, PREVIEW_HEIGHT};
use shaders;

//----------------------------------------------------------------------------

pub struct WindowController<'a> {
    window: &'a mut GlutinFacade,
    buffer_width: u32,
    buffer_height: u32,
    chan_wc_to_engine: Option<Sender<EngineCommand>>,
    chan_wc_from_engine: Option<Receiver<EngineStatus>>,
    chan_engine_to_wc: Option<Sender<EngineStatus>>,
    chan_engine_from_wc: Option<Receiver<EngineCommand>>,
    image_buf: image::DynamicImage,
    shader_program: glium::Program,
    texture: glium::Texture2d,
}

impl<'a> WindowController<'a> {
    pub fn new(window: &'a mut GlutinFacade) -> WindowController<'a> {

        // WindowController === Engine
        // (Sender<T>, Receiver<T>)
        // x_to_y == sender
        // x_from_y == receiver
        let (chan_wc_to_engine, chan_engine_from_wc) = channel();
        let (chan_engine_to_wc, chan_wc_from_engine) = channel();

        // Size image buffer to frame
        let (w, h) =  window.get_framebuffer_dimensions();
        println!("framebuffer: w={} h={}", w, h);

        let mut image_buf = image::DynamicImage::new_rgb8(w, h);

        // Load sample image into teture for render test
        let image_dimensions = (w, h);
        let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image_buf.raw_pixels(), image_dimensions);
        let texture = glium::texture::Texture2d::new(window, image).unwrap();

        // Build shaders
        let program = glium::Program::from_source(window,
                                                  shaders::VERTEX_SHADER_SOURCE,
                                                  shaders::FRAGMENT_SHADER_SOURCE,
                                                  None).unwrap();

        let mut wc = WindowController { window: window,
                                        buffer_width: w,
                                        buffer_height: h,
                                        chan_wc_to_engine: Some(chan_wc_to_engine),
                                        chan_wc_from_engine: Some(chan_wc_from_engine),
                                        chan_engine_to_wc: Some(chan_engine_to_wc),
                                        chan_engine_from_wc: Some(chan_engine_from_wc),
                                        image_buf: image_buf,
                                        shader_program: program,
                                        texture: texture,
        };

        wc
    }

    pub fn run(&mut self) {

        self.start_engine();

        loop {
            while let Some(ev) = self.window.poll_events().next() {
                self.handle_window_event(ev);
            }
            self.maybe_update_display();
            self.draw();
        }
    }

    pub fn draw(&mut self) {

        let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            tex: &self.texture,
        };

        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
            texcoord: [f32; 2],
        }

        implement_vertex!(Vertex, position, texcoord);

        let vertex1 = Vertex { position: [-0.5, -0.5], texcoord: [1.0, 1.0] };
        let vertex2 = Vertex { position: [ 0.0,  0.5], texcoord: [0.0, 1.0] };
        let vertex3 = Vertex { position: [ 0.5, -0.25],texcoord: [1.0, 0.0] };
        let shape = vec![vertex1, vertex2, vertex3];

        let vertex_buffer = glium::VertexBuffer::new(self.window, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        let mut target = self.window.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);
        target.draw(&vertex_buffer, &indices, &self.shader_program, &uniforms,
                  &Default::default()).unwrap();
        target.finish().unwrap();
    }

    pub fn start_engine(&mut self) {

        let engine_progress_ch = self.chan_engine_to_wc.take().expect("no engine_to_wc chan");
        let engine_cmd_ch = self.chan_engine_from_wc.take().expect("no engine_from_wc chan");

        let (w,h) = (self.buffer_width, self.buffer_height);

        thread::spawn(move || {

            let mut engine = MandelEngine::new(w, h);
            engine.serve(&engine_cmd_ch, &engine_progress_ch);
        });

        let cmd_ch = self.chan_wc_to_engine.as_ref().expect("no chan_wc_to_engine");

        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
        cmd_ch.send(EngineCommand::Render(RenderType::FullRender));
    }

    pub fn maybe_update_display(&mut self) {
        match self.chan_wc_from_engine {
            Some(ref ch) => {
                let status_msg = ch.try_recv();
                match status_msg {
                    Ok(status) =>
                        match status {
                            EngineStatus::Startup => println!("Startup..."),
                            EngineStatus::Processing(progress) => println!("Processing {}", progress),
                            EngineStatus::RenderComplete(typ, img) => {
                                println!("Render Complete!");
                                //self.image = Some(img);
                                let imgbuf = img.as_ptr();
                                match typ {
                                    RenderType::FullRender => {
                                        println!("fullRender {} {}", self.buffer_width, self.buffer_height);
                                        // TODO: update image and texture (full size)
                                    },
                                    RenderType::PreviewRender => {
                                        println!("Preview {} {}", PREVIEW_WIDTH, PREVIEW_HEIGHT);
                                        // TODO: update image and texture (preview size)
                                    },
                                };
                            },
                            EngineStatus::Error(code) => println!("Error {}", code),
                        },
                    _ => ()
                }
            },
            None => (),
        }
    }

    pub fn handle_window_event(&mut self, event: glium::glutin::Event) {
        println!("handle_window_event: {:?}", event);
        let cmd_ch = self.chan_wc_to_engine.as_ref().expect("no chan_wc_to_engine");
        match event {
            Event::KeyboardInput(ElementState::Released, _, Some(keycode)) => {
                match keycode {
                    VirtualKeyCode::Space => {
                        cmd_ch.send(EngineCommand::Render(RenderType::FullRender));
                    },
                    VirtualKeyCode::Equals => {
                        cmd_ch.send(EngineCommand::ZoomIn);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    VirtualKeyCode::Minus => {
                        cmd_ch.send(EngineCommand::ZoomOut);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    VirtualKeyCode::Left => {
                        cmd_ch.send(EngineCommand::PanLeft);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    VirtualKeyCode::Right => {
                        cmd_ch.send(EngineCommand::PanRight);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    VirtualKeyCode::Up => {
                        cmd_ch.send(EngineCommand::PanUp);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    VirtualKeyCode::Down => {
                        cmd_ch.send(EngineCommand::PanDown);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    VirtualKeyCode::Escape => {
                        cmd_ch.send(EngineCommand::Shutdown);
//                        self.window.set_should_close(true);
                    },
                    VirtualKeyCode::S => {
                    //     match self.image {
                    //         Some(ref img) => save_as_pgm(img, self.buffer_width, self.buffer_height, "test.pgm"),
                    //         _ => (),
                    //     }
                    },
                    _ => {}
                };
            },
            _ => {},
        }
    }
}

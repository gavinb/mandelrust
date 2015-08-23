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

use std::sync::mpsc::{channel, Sender, Receiver};
use std::vec::Vec;
use std::thread;

use glium;
use glium::glutin::{Event,VirtualKeyCode,ElementState};
use glium::backend::glutin_backend::GlutinFacade;
use glium::backend::Facade;
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
    image: Option<Vec<u8>>,
}

impl<'a> WindowController<'a> {
    pub fn new(window: &'a mut GlutinFacade) -> WindowController<'a> {

        // WindowController === Engine
        // (Sender<T>, Receiver<T>)
        // x_to_y == sender
        // x_from_y == receiver
        let (chan_wc_to_engine, chan_engine_from_wc) = channel();
        let (chan_engine_to_wc, chan_wc_from_engine) = channel();

        let mut wc = WindowController { window: window,
                                        buffer_width: 0,
                                        buffer_height: 0,
                                        chan_wc_to_engine: Some(chan_wc_to_engine),
                                        chan_wc_from_engine: Some(chan_wc_from_engine),
                                        chan_engine_to_wc: Some(chan_engine_to_wc),
                                        chan_engine_from_wc: Some(chan_engine_from_wc),
                                        image: None,
        };

        let (w, h) =  wc.window.get_framebuffer_dimensions();
        wc.buffer_width = w as u32;
        wc.buffer_height = h as u32;

//        let mut imgbuf = image::ImageBuffer::new(w, h);

        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
        }

        implement_vertex!(Vertex, position);

        let vertex1 = Vertex { position: [-0.5, -0.5] };
        let vertex2 = Vertex { position: [ 0.0,  0.5] };
        let vertex3 = Vertex { position: [ 0.5, -0.25] };
        let shape = vec![vertex1, vertex2, vertex3];

        let vertex_buffer = glium::VertexBuffer::new(wc.window, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        // x,y pos | u,v tex
        // wc.vertices = vec!(
        //     -1.0, -1.0, 0.0, 0.0,
        //      1.0, -1.0, 1.0, 0.0,
        //     -1.0,  1.0, 0.0, 1.0,
        //      1.0,  1.0, 1.0, 1.0,
        //     );

        // Build shaders

        let program = glium::Program::from_source(wc.window,
                                                  shaders::VERTEX_SHADER_SOURCE,
                                                  shaders::FRAGMENT_SHADER_SOURCE,
                                                  None).unwrap();

        //

        wc
    }

    pub fn run(&mut self) {

        self.start_engine();

        loop {
            for ev in self.window.poll_events() {
                match ev {
                    glium::glutin::Event::Closed => return,
                    _ => ()
                }
            }
            self.maybe_update_display();
            self.draw();
        }
    }

    pub fn draw(&mut self) {
//        target.clear_color(0.0, 0.0, 1.0, 1.0);

        // let uniforms = uniform! {
        //     matrix: [
        //         [1.0, 0.0, 0.0, 0.0],
        //         [0.0, 1.0, 0.0, 0.0],
        //         [0.0, 0.0, 1.0, 0.0],
        //         [0.0, 0.0, 0.0, 1.0],
        //     ],
        //     tex: &texture,
        // };

        // let mut target = self.window.draw();
        // target.clear_color(0.0, 0.0, 1.0, 1.0);
        // target.draw(&vertex_buffer, &indices, &program, &glium::uniforms::EmptyUniforms,
        //     &Default::default()).unwrap();
        // target.finish().unwrap();


        // target.draw(&vertex_buffer, &indices, &program, &uniforms,
        //           &Default::default()).unwrap();
        // target.finish().unwrap();
    }

    fn uninit(&mut self) {
    }

    pub fn start_engine(&mut self) {

        let engine_progress_ch = self.chan_engine_to_wc.take().expect("no engine_to_wc chan");
        let engine_cmd_ch = self.chan_engine_from_wc.take().expect("no engine_from_wc chan");

        let (w,h) = (self.buffer_width, self.buffer_height);

        thread::spawn(move || {

            let mut engine = MandelEngine::new(w, h);
            engine.serve(&engine_cmd_ch, &engine_progress_ch);
        });

        let cmd_ch = self.chan_wc_to_engine.take().expect("no wc_to_engine chan");

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
                                        // gl::BindTexture(gl::TEXTURE_2D, self.texture_ids[0]);
                                        // gl::TexSubImage2D(gl::TEXTURE_2D, 0,
                                        //                       0, 0,
                                        //                       self.buffer_width as i32, self.buffer_height as i32,
                                        //                       gl::RGB as u32, gl::UNSIGNED_BYTE, mem::transmute(imgbuf));
                                    },
                                    RenderType::PreviewRender => {
                                        println!("Preview {} {}", PREVIEW_WIDTH, PREVIEW_HEIGHT);
                                        // gl::BindTexture(gl::TEXTURE_2D, self.texture_ids[1]);
                                        // gl::TexSubImage2D(gl::TEXTURE_2D, 0,
                                        //                       0, 0,
                                        //                       PREVIEW_WIDTH, PREVIEW_HEIGHT,
                                        //                       gl::RGB as u32, gl::UNSIGNED_BYTE, mem::transmute(imgbuf));
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
        let cmd_ch = self.chan_wc_to_engine.take().expect("no chan_wc_to_engine");
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
                    // VirtualKeyCode::S => {
                    //     match self.image {
                    //         Some(ref img) => save_as_pgm(img, self.buffer_width, self.buffer_height, "test.pgm"),
                    //         _ => (),
                    //     }
                    // },
                    // VirtualKeyCode::R => {
                    //     // Resize should cause the window to "refresh"
                    //     let (window_width, window_height) = self.window.get_size();
                    //     self.window.set_size(window_width + 1, window_height);
                    //     self.window.set_size(window_width, window_height);
                    // },
                    _ => {}
                };
            },
            _ => {},
        }
    }
}

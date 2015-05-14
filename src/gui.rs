//============================================================================
//
// A simple Mandelbrot image generator in Rust
//
// GLFW GUI Front-end
//
// Copyright (c) 2014 Gavin Baker <gavinb@antonym.org>
// Published under the MIT license
//
//============================================================================

use glfw;
use glfw::Context;

use std::sync::mpsc::{channel, Sender, Receiver};
use std::vec::Vec;
use std::fs::File;
use std::path::Path;
use std::thread;
use std::mem;

use protocol::{RenderType, EngineStatus, EngineCommand, PREVIEW_WIDTH, PREVIEW_HEIGHT};

use engine::MandelEngine;

mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

//----------------------------------------------------------------------------

static vertex_shader_source: &'static str = "

#version 150

in vec2 position;
in vec2 texcoord;

out vec2 Texcoord;

void main()
{
    gl_Position = vec4(position, 0.0, 1.0);
    Texcoord = texcoord;
}
";

static fragment_shader_source: &'static str = "

#version 150

in vec2 Texcoord;

out vec4 outColor;

uniform sampler2D tex;

void main()
{
    outColor = texture(tex, Texcoord) * vec4(8,8,8,1);
}
";

//----------------------------------------------------------------------------

pub struct WindowController<'a> {
    window: &'a glfw::Window,
    vertices: Vec<f32>,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
    vertex_shader: gl::types::GLuint,
    fragment_shader: gl::types::GLuint,
    shader_program: gl::types::GLuint,
    texture_ids: Vec<gl::types::GLuint>,
    uni_color: gl::types::GLint,
    buffer_width: u32,
    buffer_height: u32,
    chan_wc_to_engine: Option<Sender<EngineCommand>>,
    chan_wc_from_engine: Option<Receiver<EngineStatus>>,
    chan_engine_to_wc: Option<Sender<EngineStatus>>,
    chan_engine_from_wc: Option<Receiver<EngineCommand>>,
    image: Option<Vec<u8>>,
}

impl<'a> WindowController<'a> {
    pub fn new(window: &'a glfw::Window) -> WindowController<'a> {

        // WindowController === Engine
        // (Sender<T>, Receiver<T>)
        let (chan_wc_to_engine, chan_engine_from_wc) = channel();
        let (chan_engine_to_wc, chan_wc_from_engine) = channel();

        let mut wc = WindowController { window: window, 
                                        vao: 0,
                                        vbo: 0,
                                        vertices: vec!(),
                                        vertex_shader: 0,
                                        fragment_shader: 0,
                                        shader_program: 0,
                                        texture_ids: vec!(),
                                        uni_color: 0,
                                        buffer_width: 0,
                                        buffer_height: 0,
                                        chan_wc_to_engine: Some(chan_wc_to_engine),
                                        chan_wc_from_engine: Some(chan_wc_from_engine),
                                        chan_engine_to_wc: Some(chan_engine_to_wc),
                                        chan_engine_from_wc: Some(chan_engine_from_wc),
                                        image: None,
        };

        let (w, h) =  wc.window.get_framebuffer_size();
        wc.buffer_width = w as u32;
        wc.buffer_height = h as u32;

        gl::Viewport(0, 0, wc.buffer_width as i32, wc.buffer_height as i32);

        println!("Viewport: {} x {}", wc.buffer_width, wc.buffer_height);

        // x,y pos | u,v tex
        wc.vertices = vec!(
            -1.0, -1.0, 0.0, 0.0,
             1.0, -1.0, 1.0, 0.0,
            -1.0,  1.0, 0.0, 1.0,
             1.0,  1.0, 1.0, 1.0,
            );

        // VAO

        gl::GenVertexArrays(1, &mut wc.vao);
        gl::BindVertexArray(wc.vao);

        // VBO

        gl::GenBuffers(1, &mut wc.vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, wc.vbo);
        gl::BufferData(gl::ARRAY_BUFFER, 16*4, mem::transmute(&wc.vertices[0]), gl::STATIC_DRAW);

        // Vertex Shader

        wc.vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);

        if wc.vertex_shader == 0 {
            panic!("Create v.shader failed");
        }

        gl::ShaderSource(wc.vertex_shader, 1, mem::transmute(vec![vertex_shader_source.as_ptr()].as_ptr()), &1);

        let err = gl::GetError();
        if err != 0 {
            panic!("glShaderSource.v err 0x{:x}", err);
        }

        gl::CompileShader(wc.vertex_shader);

        let err = gl::GetError();
        if err != 0 {
            panic!("glCompileShader.f err 0x{:x}", err);
        }

        let status: i32;
        gl::GetShaderiv(wc.vertex_shader, gl::COMPILE_STATUS, &mut status);

        if status != gl::TRUE as i32 {
            let log_len: gl::types::GLsizei = 256;
            let log = Vec::<gl::types::GLchar>::with_capacity(log_len as usize);
            let log_ptr: *const i8 = mem::transmute(&log.as_ptr());
            gl::GetShaderInfoLog(wc.vertex_shader, log_len, &mut log_len, &log_ptr);
            panic!("glCompileShader.v err 0x{:x}: {}", status, log);
        }

        // Fragment Shader

        wc.fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);

        if wc.vertex_shader == 0 {
            panic!("Create f.shader failed");
        }

        gl::ShaderSource(wc.fragment_shader, 1, mem::transmute(vec![fragment_shader_source.as_ptr()].as_ptr()), &1);

        let err = gl::GetError();
        if err != 0 {
            panic!("glShaderSource.f -> 0x{:x}", err);
        }

        gl::CompileShader(wc.fragment_shader);

        let err = gl::GetError();
        if err != 0 {
            panic!("glCompileShader.v err 0x{:x}", err);
        }

        let status = gl::GetShaderiv(wc.fragment_shader, gl::COMPILE_STATUS);

        if status != gl::TRUE as i32 {
            let log_len: gl::types::GLsizei = 256;
            let log = Vec::<gl::types::GLchar>::with_capacity(log_len as usize);
            gl::GetShaderInfoLog(wc.fragment_shader, log_len, &mut log_len, &mut log);
            panic!("glCompileShader.f err 0x{:x}: {}", status, log);
        }

        // Link

        wc.shader_program = gl::CreateProgram();

        if wc.shader_program == 0 {
            panic!("glCreateProgram failed");
        }

        gl::AttachShader(wc.shader_program, wc.vertex_shader);
        gl::AttachShader(wc.shader_program, wc.fragment_shader);

        let err = gl::GetError();
        if err != 0 {
            panic!("glAttachShader err 0x{:x}", err);
        }

        gl::LinkProgram(wc.shader_program);

        let err = gl::GetError();
        if err != 0 {
            panic!("glLinkProgram err 0x{:x}", err);
        }

        let status: gl::types::GLint = 0;
        gl::GetProgramiv(wc.shader_program, gl::LINK_STATUS, &status);

        if status != gl::TRUE as i32 {
            let log = gl::GetShaderInfoLog(wc.shader_program);
            panic!("glLinkProgram err {}: {}", status, log);
        }

        gl::UseProgram(wc.shader_program);

        let err = gl::GetError();
        if err != 0 {
            let log = gl::GetProgramInfoLog(wc.shader_program);
            panic!("glUseProgram error: {}", log);
        }

        // Attributes

        let pos_attrib = gl::GetAttribLocation(wc.shader_program, "position");
        gl::EnableVertexAttribArray(pos_attrib as gl::types::GLuint);
        gl::VertexAttribPointer(pos_attrib as gl::types::GLuint, 2, false, 4*4, 0);

        let tex_attrib = gl::GetAttribLocation(wc.shader_program, "texcoord");
        gl::EnableVertexAttribArray(tex_attrib as gl::types::GLuint);
        gl::VertexAttribPointer(tex_attrib as gl::types::GLuint, 2, false, 4*4, 2*4);

        let err = gl::GetError();
        if err != 0 {
            panic!("attrib err = {:x}", err);
        }

        // Setup textures

        wc.texture_ids = gl::GenTextures(2);

        // Full tex
        gl::BindTexture(gl::TEXTURE_2D, wc.texture_ids[0]);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32,
                          wc.buffer_width as i32, wc.buffer_height as i32, 0,
                          gl::RGB as u32, gl::UNSIGNED_BYTE, None);

        // Preview tex
        gl::BindTexture(gl::TEXTURE_2D, wc.texture_ids[1]);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32,
                          PREVIEW_WIDTH, PREVIEW_HEIGHT, 0,
                          gl::RGB as u32, gl::UNSIGNED_BYTE, None);

        let err = gl::GetError();
        if err != 0 {
            panic!("tex err = {:x}", err);
        }

        //

        wc
    }

    pub fn draw(&self) {

        // Clear

        gl::ClearColor(0.4, 0.4, 0.4, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);

        // Draw!

        let err = gl::GetError();
        if err != 0 {
            println!("draw err = 0x{:x}", err);
        }

        // Render texture
        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

        let err = gl::GetError();
        if err != 0 {
            println!("drawz err = 0x{:x}", err);
        }

        self.window.swap_buffers();
    }

    fn uninit(&mut self) {

        // Cleanup

        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::DeleteTextures(1, &self.texture_ids[0]);

        gl::DeleteProgram(self.shader_program);
        gl::DeleteShader(self.fragment_shader);
        gl::DeleteShader(self.vertex_shader);

        gl::DeleteBuffers(self.vbo);

        gl::DeleteVertexArrays(self.vao);
    }

    pub fn start_engine(&'a mut self) {

        let progress_ch = self.chan_engine_to_wc.take().expect("no engine_to_wc chan");
        let cmd_ch = self.chan_engine_from_wc.take().expect("no engine_from_wc chan");

        let (w,h) = (self.buffer_width, self.buffer_height);

//        task::spawn( || {

            let mut engine = MandelEngine::new(w, h);
            engine.serve(&cmd_ch, &progress_ch);
//        });

        let cmd_ch = self.chan_wc_to_engine.unwrap();
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
                                let imgbuf = img.as_slice().as_ptr();
                                match typ {
                                    RenderType::FullRender => {
                                        println!("fullRender {} {}", self.buffer_width, self.buffer_height);
                                        gl::BindTexture(gl::TEXTURE_2D, self.texture_ids[0]);
                                        gl::TexSubImage2D(gl::TEXTURE_2D, 0,
                                                              0, 0,
                                                              self.buffer_width as i32, self.buffer_height as i32,
                                                              gl::RGB as u32, gl::UNSIGNED_BYTE, mem::transmute(imgbuf));
                                    },
                                    RenderType::PreviewRender => {
                                        println!("Preview {} {}", PREVIEW_WIDTH, PREVIEW_HEIGHT);
                                        gl::BindTexture(gl::TEXTURE_2D, self.texture_ids[1]);
                                        gl::TexSubImage2D(gl::TEXTURE_2D, 0,
                                                              0, 0,
                                                              PREVIEW_WIDTH, PREVIEW_HEIGHT,
                                                              gl::RGB as u32, gl::UNSIGNED_BYTE, mem::transmute(imgbuf));
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

    pub fn handle_window_event(&self, window: &glfw::Window, (time, event): (f64, glfw::WindowEvent)) {
        let cmd_ch = self.chan_wc_to_engine.unwrap();
        match event {

            glfw::WindowEvent::Close => println!("Time: {}, Window close requested.", time),

            glfw::WindowEvent::Key(key, scancode, action, mods) => {
                println!("Time: {}, Key: {}, ScanCode: {}, Action: {}, Modifiers: [{}]", time, key, scancode, action, mods);
                match (key, action) {
                    (glfw::Key::Space, glfw::Action::Press) => {
                        cmd_ch.send(EngineCommand::Render(RenderType::FullRender));
                    },
                    (glfw::Key::Equal, glfw::Action::Press) => {
                        cmd_ch.send(EngineCommand::ZoomIn);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    (glfw::Key::Minus, glfw::Action::Press) => {
                        cmd_ch.send(EngineCommand::ZoomOut);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    (glfw::Key::Left, glfw::Action::Press) => {
                        cmd_ch.send(EngineCommand::PanLeft);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    (glfw::Key::Right, glfw::Action::Press) => {
                        cmd_ch.send(EngineCommand::PanRight);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    (glfw::Key::Up, glfw::Action::Press) => {
                        cmd_ch.send(EngineCommand::PanUp);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    (glfw::Key::Down, glfw::Action::Press) => {
                        cmd_ch.send(EngineCommand::PanDown);
                        cmd_ch.send(EngineCommand::Render(RenderType::PreviewRender));
                    },
                    (glfw::Key::Escape, glfw::Action::Press) => {
                        cmd_ch.send(EngineCommand::Shutdown);
                        window.set_should_close(true);
                    },
                    (glfw::Key::S, glfw::Action::Press) => {
                        match self.image {
                            Some(ref img) => save_as_pgm(img, self.buffer_width, self.buffer_height, "test.pgm"),
                            _ => (),
                        }
                    },
                    (glfw::Key::R, glfw::Action::Press) => {
                        // Resize should cause the window to "refresh"
                        let (window_width, window_height) = window.get_size();
                        window.set_size(window_width + 1, window_height);
                        window.set_size(window_width, window_height);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn save_as_pgm(img: &Vec<u8>, width: u32, height: u32, filename: &str) {
        let mut file = File::create(&Path::new(filename));
        file.write(b"P6\n");
        file.write(format!("{} {}\n255\n", width, height).as_bytes());
        file.write(img.slice(0, img.capacity()));
}

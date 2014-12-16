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

use gleam::gl;

use glfw;
use glfw::Context;

use std::comm::{Sender, Receiver, channel};
use std::vec::Vec;
use std::io::File;
use std::path::Path;
use std::task;

use protocol::{RenderType, EngineStatus, EngineCommand, PREVIEW_WIDTH, PREVIEW_HEIGHT};

use engine::MandelEngine;

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
    vao: Vec<gl::GLuint>,
    vbo: Vec<gl::GLuint>,
    vertex_shader: gl::GLuint,
    fragment_shader: gl::GLuint,
    shader_program: gl::GLuint,
    texture_ids: Vec<gl::GLuint>,
    uni_color: gl::GLint,
    buffer_width: uint,
    buffer_height: uint,
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
                                        vao: vec!(0),
                                        vbo: vec!(0),
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
        wc.buffer_width = w as uint;
        wc.buffer_height = h as uint;

        gl::viewport(0, 0, wc.buffer_width as i32, wc.buffer_height as i32);

        println!("Viewport: {} x {}", wc.buffer_width, wc.buffer_height);

        // x,y pos | u,v tex
        wc.vertices = vec!(
            -1.0, -1.0, 0.0, 0.0,
             1.0, -1.0, 1.0, 0.0,
            -1.0,  1.0, 0.0, 1.0,
             1.0,  1.0, 1.0, 1.0,
            );

        // VAO

        wc.vao = gl::gen_vertex_arrays(1);
        gl::bind_vertex_array(wc.vao[0]);

        // VBO

        wc.vbo = gl::gen_buffers(1);
        gl::bind_buffer(gl::ARRAY_BUFFER, wc.vbo[0]);
        gl::buffer_data(gl::ARRAY_BUFFER, wc.vertices, gl::STATIC_DRAW);

        // Vertex Shader

        wc.vertex_shader = gl::create_shader(gl::VERTEX_SHADER);

        if wc.vertex_shader == 0 {
            panic!("Create v.shader failed");
        }

        gl::shader_source(wc.vertex_shader, [vertex_shader_source.to_owned().as_bytes()]);

        let err = gl::get_error();
        if err != 0 {
            panic!("glShaderSource.v err 0x{:x}", err);
        }

        gl::compile_shader(wc.vertex_shader);

        let err = gl::get_error();
        if err != 0 {
            panic!("glCompileShader.f err 0x{:x}", err);
        }

        let status = gl::get_shader_iv(wc.vertex_shader, gl::COMPILE_STATUS);

        if status != gl::TRUE as i32 {
            let log = gl::get_shader_info_log(wc.vertex_shader);
            panic!("glCompileShader.v err 0x{:x}: {}", status, log);
        }

        // Fragment Shader

        wc.fragment_shader = gl::create_shader(gl::FRAGMENT_SHADER);

        if wc.vertex_shader == 0 {
            panic!("Create f.shader failed");
        }

        gl::shader_source(wc.fragment_shader, [fragment_shader_source.to_owned().as_bytes()]);

        let err = gl::get_error();
        if err != 0 {
            panic!("glShaderSource.f -> 0x{:x}", err);
        }

        gl::compile_shader(wc.fragment_shader);

        let err = gl::get_error();
        if err != 0 {
            panic!("glCompileShader.v err 0x{:x}", err);
        }

        let status = gl::get_shader_iv(wc.fragment_shader, gl::COMPILE_STATUS);

        if status != gl::TRUE as i32 {
            let log = gl::get_shader_info_log(wc.fragment_shader);
            panic!("glCompileShader.f err 0x{:x}: {}", status, log);
        }

        // Link

        wc.shader_program = gl::create_program();

        if wc.shader_program == 0 {
            panic!("glCreateProgram failed");
        }

        gl::attach_shader(wc.shader_program, wc.vertex_shader);
        gl::attach_shader(wc.shader_program, wc.fragment_shader);

        let err = gl::get_error();
        if err != 0 {
            panic!("glAttachShader err 0x{:x}", err);
        }

        gl::link_program(wc.shader_program);

        let err = gl::get_error();
        if err != 0 {
            panic!("glLinkProgram err 0x{:x}", err);
        }

        let status = gl::get_program_iv(wc.shader_program, gl::LINK_STATUS);

        if status != gl::TRUE as i32 {
            let log = gl::get_shader_info_log(wc.shader_program);
            panic!("glLinkProgram err {}: {}", status, log);
        }

        gl::use_program(wc.shader_program);

        let err = gl::get_error();
        if err != 0 {
            let log = gl::get_program_info_log(wc.shader_program);
            panic!("glUseProgram error: {}", log);
        }

        // Attributes

        let pos_attrib = gl::get_attrib_location(wc.shader_program, "position");
        gl::enable_vertex_attrib_array(pos_attrib as gl::GLuint);
        gl::vertex_attrib_pointer_f32(pos_attrib as gl::GLuint, 2, false, 4*4, 0);

        let tex_attrib = gl::get_attrib_location(wc.shader_program, "texcoord");
        gl::enable_vertex_attrib_array(tex_attrib as gl::GLuint);
        gl::vertex_attrib_pointer_f32(tex_attrib as gl::GLuint, 2, false, 4*4, 2*4);

        let err = gl::get_error();
        if err != 0 {
            panic!("attrib err = {:x}", err);
        }

        // Setup textures

        wc.texture_ids = gl::gen_textures(2);

        // Full tex
        gl::bind_texture(gl::TEXTURE_2D, wc.texture_ids[0]);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::tex_image_2d(gl::TEXTURE_2D, 0, gl::RGB as i32,
                          wc.buffer_width as i32, wc.buffer_height as i32, 0,
                          gl::RGB as u32, gl::UNSIGNED_BYTE, None);

        // Preview tex
        gl::bind_texture(gl::TEXTURE_2D, wc.texture_ids[1]);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::tex_parameter_i(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::tex_image_2d(gl::TEXTURE_2D, 0, gl::RGB as i32,
                          PREVIEW_WIDTH, PREVIEW_HEIGHT, 0,
                          gl::RGB as u32, gl::UNSIGNED_BYTE, None);

        let err = gl::get_error();
        if err != 0 {
            panic!("tex err = {:x}", err);
        }

        //

        wc
    }

    pub fn draw(&self) {

        // Clear

        gl::clear_color(0.4, 0.4, 0.4, 1.0);
        gl::clear(gl::COLOR_BUFFER_BIT);

        // Draw!

        let err = gl::get_error();
        if err != 0 {
            println!("draw err = 0x{:x}", err);
        }

        // Render texture
        gl::draw_arrays(gl::TRIANGLE_STRIP, 0, 4);

        let err = gl::get_error();
        if err != 0 {
            println!("drawz err = 0x{:x}", err);
        }

        self.window.swap_buffers();
    }

    fn uninit(&mut self) {

        // Cleanup

        gl::bind_texture(gl::TEXTURE_2D, 0);
        gl::delete_textures(self.texture_ids);

        gl::delete_program(self.shader_program);
        gl::delete_shader(self.fragment_shader);
        gl::delete_shader(self.vertex_shader);

        gl::delete_buffers(self.vbo);

        // @todo Missing from bindings
        //gl::delete_vertex_arrays(vao);
    }

    pub fn start_engine(&mut self) {

        let progress_ch = self.chan_engine_to_wc.take().expect("no engine_to_wc chan");
        let cmd_ch = self.chan_engine_from_wc.take().expect("no engine_from_wc chan");

        let (w,h) = (self.buffer_width, self.buffer_height);

        task::spawn( || {

            let mut engine = MandelEngine::new(w, h);
            engine.serve(&cmd_ch, &progress_ch);
        });

        let cmd_ch = self.chan_wc_to_engine.get_ref();
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
                                let imgbuf = Some(img.as_slice());
                                match typ {
                                    RenderType::FullRender => {
                                        println!("fullRender {} {}", self.buffer_width, self.buffer_height);
                                        gl::bind_texture(gl::TEXTURE_2D, self.texture_ids[0]);
                                        gl::tex_sub_image_2d(gl::TEXTURE_2D, 0,
                                                              0, 0,
                                                              self.buffer_width as i32, self.buffer_height as i32,
                                                              gl::RGB as u32, gl::UNSIGNED_BYTE, imgbuf);
                                    },
                                    RenderType::PreviewRender => {
                                        println!("Preview {} {}", PREVIEW_WIDTH, PREVIEW_HEIGHT);
                                        gl::bind_texture(gl::TEXTURE_2D, self.texture_ids[1]);
                                        gl::tex_sub_image_2d(gl::TEXTURE_2D, 0,
                                                              0, 0,
                                                              PREVIEW_WIDTH, PREVIEW_HEIGHT,
                                                              gl::RGB as u32, gl::UNSIGNED_BYTE, imgbuf);
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
        let cmd_ch = self.chan_wc_to_engine.get_ref();
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

fn save_as_pgm(img: &Vec<u8>, width: uint, height: uint, filename: &str) {
        let mut file = File::create(&Path::new(filename));
        file.write(b"P6\n");
        file.write_str(format!("{} {}\n255\n", width, height));
        file.write(img.slice(0, img.capacity()));
}

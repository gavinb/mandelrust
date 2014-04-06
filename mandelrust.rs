//============================================================================
//
// A simple Mandelbrot image generator in Rust
//
// Copyright (c) 2014 Gavin Baker <gavinb@antonym.org>
// Published under the MIT license
//
// Packages required:
//
// - OpenGLES
// - GLFW
//
//============================================================================

#![license = "MIT"]

extern crate native;
extern crate num;

extern crate opengles;
extern crate glfw;

use glfw::Context;

use opengles::gl2;

use std::comm::{Sender, Receiver, Data, channel};
use std::vec::Vec;
use std::io::File;
use std::path::Path;

static PREVIEW_WIDTH: i32 = 256;
static PREVIEW_HEIGHT: i32 = 256;

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

#[deriving(Show)]
enum RenderType {
    PreviewRender,
    FullRender,
}

//----------------------------------------------------------------------------

#[deriving(Show)]
enum EngineStatus {
    Startup,
    Processing(uint),
    RenderComplete(RenderType, Vec<u8>),
    Error(uint)
}

//----------------------------------------------------------------------------

#[deriving(Show)]
enum EngineCommand {
    UpdateRegion(f32, f32, f32, f32),
    ZoomIn,
    ZoomOut,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    Render(RenderType),
    Shutdown,
}

//----------------------------------------------------------------------------

struct WindowController<'a> {
    window: &'a glfw::Window,
    vertices: ~[f32],
    vao: ~[gl2::GLuint],
    vbo: ~[gl2::GLuint],
    vertex_shader: gl2::GLuint,
    fragment_shader: gl2::GLuint,
    shader_program: gl2::GLuint,
    texture_ids: ~[gl2::GLuint],
    uni_color: gl2::GLint,
    buffer_width: uint,
    buffer_height: uint,
    chan_wc_to_engine: Option<Sender<EngineCommand>>,
    chan_wc_from_engine: Option<Receiver<EngineStatus>>,
    chan_engine_to_wc: Option<Sender<EngineStatus>>,
    chan_engine_from_wc: Option<Receiver<EngineCommand>>,
    image: Option<Vec<u8>>,
}

impl<'a> WindowController<'a> {
    fn new(window: &'a glfw::Window) -> WindowController<'a> {

        // WindowController === Engine
        // (Sender<T>, Receiver<T>)
        let (chan_wc_to_engine, chan_engine_from_wc) = channel();
        let (chan_engine_to_wc, chan_wc_from_engine) = channel();

        let mut wc = WindowController { window: window, 
                                        vao: ~[0],
                                        vbo: ~[0],
                                        vertices: ~[],
                                        vertex_shader: 0,
                                        fragment_shader: 0,
                                        shader_program: 0,
                                        texture_ids: ~[],
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

        gl2::viewport(0, 0, wc.buffer_width as i32, wc.buffer_height as i32);

        println!("Viewport: {} x {}", wc.buffer_width, wc.buffer_height);

        // x,y pos | u,v tex
        wc.vertices = ~[
            -1.0, -1.0, 0.0, 0.0,
             1.0, -1.0, 1.0, 0.0,
            -1.0,  1.0, 0.0, 1.0,
             1.0,  1.0, 1.0, 1.0,
            ];

        // VAO

        wc.vao = gl2::gen_vertex_arrays(1);
        gl2::bind_vertex_array(wc.vao[0]);

        // VBO

        wc.vbo = gl2::gen_buffers(1);
        gl2::bind_buffer(gl2::ARRAY_BUFFER, wc.vbo[0]);
        gl2::buffer_data(gl2::ARRAY_BUFFER, wc.vertices, gl2::STATIC_DRAW);

        // Vertex Shader

        wc.vertex_shader = gl2::create_shader(gl2::VERTEX_SHADER);

        if wc.vertex_shader == 0 {
            fail!("Create v.shader failed");
        }

        gl2::shader_source(wc.vertex_shader, [vertex_shader_source.to_owned().as_bytes()]);

        let err = gl2::get_error();
        if err != 0 {
            fail!("glShaderSource.v err 0x{:x}", err);
        }

        gl2::compile_shader(wc.vertex_shader);

        let err = gl2::get_error();
        if err != 0 {
            fail!("glCompileShader.f err 0x{:x}", err);
        }

        let status = gl2::get_shader_iv(wc.vertex_shader, gl2::COMPILE_STATUS);

        if status != gl2::TRUE as i32 {
            let log = gl2::get_shader_info_log(wc.vertex_shader);
            fail!("glCompileShader.v err 0x{:x}: {}", status, log);
        }

        // Fragment Shader

        wc.fragment_shader = gl2::create_shader(gl2::FRAGMENT_SHADER);

        if wc.vertex_shader == 0 {
            fail!("Create f.shader failed");
        }

        gl2::shader_source(wc.fragment_shader, [fragment_shader_source.to_owned().as_bytes()]);

        let err = gl2::get_error();
        if err != 0 {
            fail!("glShaderSource.f -> 0x{:x}", err);
        }

        gl2::compile_shader(wc.fragment_shader);

        let err = gl2::get_error();
        if err != 0 {
            fail!("glCompileShader.v err 0x{:x}", err);
        }

        let status = gl2::get_shader_iv(wc.fragment_shader, gl2::COMPILE_STATUS);

        if status != gl2::TRUE as i32 {
            let log = gl2::get_shader_info_log(wc.fragment_shader);
            fail!("glCompileShader.f err 0x{:x}: {}", status, log);
        }

        // Link

        wc.shader_program = gl2::create_program();

        if wc.shader_program == 0 {
            fail!("glCreateProgram failed");
        }

        gl2::attach_shader(wc.shader_program, wc.vertex_shader);
        gl2::attach_shader(wc.shader_program, wc.fragment_shader);

        let err = gl2::get_error();
        if err != 0 {
            fail!("glAttachShader err 0x{:x}", err);
        }

        gl2::link_program(wc.shader_program);

        let err = gl2::get_error();
        if err != 0 {
            fail!("glLinkProgram err 0x{:x}", err);
        }

        let status = gl2::get_program_iv(wc.shader_program, gl2::LINK_STATUS);

        if status != gl2::TRUE as i32 {
            let log = gl2::get_shader_info_log(wc.shader_program);
            fail!("glLinkProgram err {}: {}", status, log);
        }

        gl2::use_program(wc.shader_program);

        let err = gl2::get_error();
        if err != 0 {
            let log = gl2::get_program_info_log(wc.shader_program);
            fail!("glUseProgram error: {}", log);
        }

        // Attributes

        let pos_attrib = gl2::get_attrib_location(wc.shader_program, "position");
        gl2::enable_vertex_attrib_array(pos_attrib as gl2::GLuint);
        gl2::vertex_attrib_pointer_f32(pos_attrib as gl2::GLuint, 2, false, 4*4, 0);

        let tex_attrib = gl2::get_attrib_location(wc.shader_program, "texcoord");
        gl2::enable_vertex_attrib_array(tex_attrib as gl2::GLuint);
        gl2::vertex_attrib_pointer_f32(tex_attrib as gl2::GLuint, 2, false, 4*4, 2*4);

        let err = gl2::get_error();
        if err != 0 {
            fail!("attrib err = {:x}", err);
        }

        // Setup textures

        wc.texture_ids = gl2::gen_textures(2);

        // Full tex
        gl2::bind_texture(gl2::TEXTURE_2D, wc.texture_ids[0]);
        gl2::tex_parameter_i(gl2::TEXTURE_2D, gl2::TEXTURE_WRAP_S, gl2::CLAMP_TO_EDGE as i32);
        gl2::tex_parameter_i(gl2::TEXTURE_2D, gl2::TEXTURE_WRAP_T, gl2::CLAMP_TO_EDGE as i32);
        gl2::tex_parameter_i(gl2::TEXTURE_2D, gl2::TEXTURE_MIN_FILTER, gl2::LINEAR as i32);
        gl2::tex_parameter_i(gl2::TEXTURE_2D, gl2::TEXTURE_MAG_FILTER, gl2::LINEAR as i32);
        gl2::tex_image_2d(gl2::TEXTURE_2D, 0, gl2::RGB as i32,
                          wc.buffer_width as i32, wc.buffer_height as i32, 0,
                          gl2::RGB as u32, gl2::UNSIGNED_BYTE, None);

        // Preview tex
        gl2::bind_texture(gl2::TEXTURE_2D, wc.texture_ids[1]);
        gl2::tex_parameter_i(gl2::TEXTURE_2D, gl2::TEXTURE_WRAP_S, gl2::CLAMP_TO_EDGE as i32);
        gl2::tex_parameter_i(gl2::TEXTURE_2D, gl2::TEXTURE_WRAP_T, gl2::CLAMP_TO_EDGE as i32);
        gl2::tex_parameter_i(gl2::TEXTURE_2D, gl2::TEXTURE_MIN_FILTER, gl2::LINEAR as i32);
        gl2::tex_parameter_i(gl2::TEXTURE_2D, gl2::TEXTURE_MAG_FILTER, gl2::LINEAR as i32);
        gl2::tex_image_2d(gl2::TEXTURE_2D, 0, gl2::RGB as i32,
                          PREVIEW_WIDTH, PREVIEW_HEIGHT, 0,
                          gl2::RGB as u32, gl2::UNSIGNED_BYTE, None);

        let err = gl2::get_error();
        if err != 0 {
            fail!("tex err = {:x}", err);
        }

        //

        wc
    }

    fn draw(&self) {

        // Clear

        gl2::clear_color(0.4, 0.4, 0.4, 1.0);
        gl2::clear(gl2::COLOR_BUFFER_BIT);

        // Draw!

        let err = gl2::get_error();
        if err != 0 {
            println!("draw err = 0x{:x}", err);
        }

        // Render texture
        gl2::draw_arrays(gl2::TRIANGLE_STRIP, 0, 4);

        let err = gl2::get_error();
        if err != 0 {
            println!("drawz err = 0x{:x}", err);
        }

        self.window.swap_buffers();
    }

    fn uninit(&mut self) {

        // Cleanup

        gl2::bind_texture(gl2::TEXTURE_2D, 0);
        gl2::delete_textures(self.texture_ids);

        gl2::delete_program(self.shader_program);
        gl2::delete_shader(self.fragment_shader);
        gl2::delete_shader(self.vertex_shader);

        gl2::delete_buffers(self.vbo);

        // @todo Missing from bindings
        //gl2::delete_vertex_arrays(vao);
    }

    fn start_engine(&mut self) {

        let progress_ch = self.chan_engine_to_wc.take().expect("no engine_to_wc chan");
        let cmd_ch = self.chan_engine_from_wc.take().expect("no engine_from_wc chan");

        let (w,h) = (self.buffer_width, self.buffer_height);

        native::task::spawn( proc() {

            let mut engine = MandelEngine::new(w, h);
            engine.serve(&cmd_ch, &progress_ch);
        });

        let cmd_ch = self.chan_wc_to_engine.get_ref();
        cmd_ch.send(Render(PreviewRender));
        cmd_ch.send(Render(FullRender));
    }

    fn maybe_update_display(&mut self) {
        match self.chan_wc_from_engine {
            Some(ref ch) => {
                let status_msg = ch.try_recv();
                match status_msg {
                    Data(status) =>
                        match status {
                            Startup => println!("Startup..."),
                            Processing(progress) => println!("Processing {}", progress),
                            RenderComplete(typ, img) => {
                                println!("Render Complete!");
                                //self.image = Some(img);
                                let imgbuf = Some(img.as_slice());
                                match typ {
                                    FullRender => {
                                        println!("fullRender {} {}", self.buffer_width, self.buffer_height);
                                        gl2::bind_texture(gl2::TEXTURE_2D, self.texture_ids[0]);
                                        gl2::tex_sub_image_2d(gl2::TEXTURE_2D, 0,
                                                              0, 0,
                                                              self.buffer_width as i32, self.buffer_height as i32,
                                                              gl2::RGB as u32, gl2::UNSIGNED_BYTE, imgbuf);
                                    },
                                    PreviewRender => {
                                        println!("Preview {} {}", PREVIEW_WIDTH, PREVIEW_HEIGHT);
                                        gl2::bind_texture(gl2::TEXTURE_2D, self.texture_ids[1]);
                                        gl2::tex_sub_image_2d(gl2::TEXTURE_2D, 0,
                                                              0, 0,
                                                              PREVIEW_WIDTH, PREVIEW_HEIGHT,
                                                              gl2::RGB as u32, gl2::UNSIGNED_BYTE, imgbuf);
                                    },
                                };
                            },
                            Error(code) => println!("Error {}", code),
                        },
                    _ => ()
                }
            },
            None => (),
        }
    }

    fn handle_window_event(&self, window: &glfw::Window, (time, event): (f64, glfw::WindowEvent)) {
        let cmd_ch = self.chan_wc_to_engine.get_ref();
        match event {

            glfw::CloseEvent => println!("Time: {}, Window close requested.", time),

            glfw::KeyEvent(key, scancode, action, mods) => {
//                println!("Time: {}, Key: {}, ScanCode: {}, Action: {}, Modifiers: [{}]", time, key, scancode, action, mods);
                match (key, action) {
                    (glfw::KeySpace, glfw::Press) => {
                        cmd_ch.send(Render(FullRender));
                    },
                    (glfw::KeyEqual, glfw::Press) => {
                        cmd_ch.send(ZoomIn);
                        cmd_ch.send(Render(PreviewRender));
                    },
                    (glfw::KeyMinus, glfw::Press) => {
                        cmd_ch.send(ZoomOut);
                        cmd_ch.send(Render(PreviewRender));
                    },
                    (glfw::KeyLeft, glfw::Press) => {
                        cmd_ch.send(PanLeft);
                        cmd_ch.send(Render(PreviewRender));
                    },
                    (glfw::KeyRight, glfw::Press) => {
                        cmd_ch.send(PanRight);
                        cmd_ch.send(Render(PreviewRender));
                    },
                    (glfw::KeyUp, glfw::Press) => {
                        cmd_ch.send(PanUp);
                        cmd_ch.send(Render(PreviewRender));
                    },
                    (glfw::KeyDown, glfw::Press) => {
                        cmd_ch.send(PanDown);
                        cmd_ch.send(Render(PreviewRender));
                    },
                    (glfw::KeyEscape, glfw::Press) => {
                        cmd_ch.send(Shutdown);
                        window.set_should_close(true);
                    },
                    (glfw::KeyS, glfw::Press) => {
                        match self.image {
                            Some(ref img) => save_as_pgm(img, self.buffer_width, self.buffer_height, "test.pgm"),
                            _ => (),
                        }
                    },
                    (glfw::KeyR, glfw::Press) => {
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
        // Save
        let filename = "mt.ppm";
        let mut file = File::create(&Path::new(filename));
        file.write(bytes!("P6\n"));
        file.write_str(format!("{} {}\n255\n", width, height));
        file.write(img.slice(0, img.capacity()));
}

//----------------------------------------------------------------------------

type RGB8 = (u8, u8, u8);

struct MandelEngine {
    buffer_width: uint,
    buffer_height: uint,
    palette: Vec<RGB8>,
    re0: f32,
    re1: f32,
    im0: f32,
    im1: f32,
}

impl MandelEngine {

    fn new(w: uint, h: uint) -> MandelEngine {

        // Init palette using hue sweep in HSV colour space
        let mut p: Vec<RGB8> = Vec::with_capacity(720);
        let s = 1.0f32; // saturation
        let v = 1.0f32; // value
        let c = v * s; // chroma
        for h in range(0, 720) { // hue
            let hp = h/60;
            let x = c * (1-(hp % 2 - 1).abs()) as f32;
            let (r,g,b) = if 0 <= hp && hp < 1 {
                (c, x, 0.0)
            } else if 1 <= hp && hp < 2 {
                (x, c, 0.0)
            } else if 2 <= hp && hp < 3 {
                (0.0, c, x)
            } else if 3 <= hp && hp < 4 {
                (0.0, x, c)
            } else if 4 <= hp && hp < 5 {
                (x, 0.0, c)
            } else if 5 <= hp && hp < 6 {
                (c, 0.0, x)
            } else {
                (0.0, 0.0, 0.0)
            };

            p.push(((r*255.0) as u8, (g*255.0) as u8, (b*255.0) as u8));
        }

        MandelEngine {
            re0: -1.4f32,
            re1:  0.6f32,
            im0: -1.0f32,
            im1:  1.0f32,
            buffer_width: w,
            buffer_height: h,
            palette: p
        }
    }

    // Rescale pixel coord (x,y) into cspace
    fn scale_coords(&self, x: uint, y: uint, w: uint, h: uint) -> (f32, f32) {
        let x0 = self.re0;
        let x1 = self.re1;
        let y0 = self.im0;
        let y1 = self.im1;

        let xx = (x as f32) / (w as f32) * (x1-x0) + x0;
        let yy = (y as f32) / (h as f32) * (y1-y0) + y0;

        (xx, yy)
    }

    fn serve(&mut self, cmd_chan: &Receiver<EngineCommand>, progress_chan: &Sender<EngineStatus>) {
        let mut running = true;
        while running {
            // pan/zoom by 10% of width
            let delta_r = ((self.re1 - self.re0)*0.1f32).abs();
            let delta_i = ((self.im1 - self.im0)*0.1f32).abs();
            println!("delta r,i {},{}", delta_r, delta_i);

            let cmd = cmd_chan.recv();
            println!("engine: command {}", cmd);
            match cmd {
                UpdateRegion(re0, re1, im0, im1) => {
                    self.re0 = re0; self.re1 = re1; self.im0 = im0; self.im1 = im1;
                },
                ZoomIn => {
                    self.re0 += delta_r;
                    self.re1 -= delta_r;
                    self.im0 += delta_i;
                    self.im1 -= delta_i;
                },
                ZoomOut => {
                    self.re0 -= delta_r;
                    self.re1 += delta_r;
                    self.im0 -= delta_i;
                    self.im1 += delta_i;
                },
                PanLeft => {
                    self.re0 -= delta_r;
                    self.re1 -= delta_r;
                },
                PanRight => {
                    self.re0 += delta_r;
                    self.re1 += delta_r;
                },
                PanUp => {
                    self.im0 += delta_i;
                    self.im1 += delta_i;
                },
                PanDown => {
                    self.im0 -= delta_i;
                    self.im1 -= delta_i;
                },
                Render(typ) => self.process(typ, progress_chan),
                Shutdown => running = false,
            }
        }

        println!("engine: shutdown");
    }

    // Evalute entire region
    fn process(&mut self, typ: RenderType, progress_chan: &Sender<EngineStatus>) {

        let (width, height) = match typ {
            PreviewRender => (PREVIEW_WIDTH as uint, PREVIEW_HEIGHT as uint),
            FullRender => (self.buffer_height, self.buffer_width),
        };

        let mut img: Vec<u8> = Vec::with_capacity(width*height*3);

        let max_iteration = 500;

        println!("+++ process {}x{} RGB8 in {} bytes", width, height, img.capacity());
        println!("            re: {}..{} im: {}..{}", self.re0, self.re1, self.im0, self.im1);

        progress_chan.send(Startup);

        // Process each pixel
        for py in range(0, height) {
            for px in range(0, width) {

                // Project pixels into Mandelbrot domain
                let (x0, y0) = self.scale_coords(px, py, width, height);

                let mut x = 0.0f32;
                let mut y = 0.0f32;
                let mut iteration = 0;

                // Iterate!
                while (x*x + y*y < 4.0) && (iteration < max_iteration) {
                    let x1 = x*x - y*y + x0;
                    y = 2.0*x*y + y0;
                    x = x1;

                    iteration += 1;
                }

                // Colour and plot
                let color = self.palette.get(iteration % 580);
                let (r,g,b) = *color;

                img.push(r);
                img.push(g);
                img.push(b);
            }
            if py % 100 == 0 {
                progress_chan.send(Processing(py));
            }
        }

        progress_chan.send(RenderComplete(typ, img));
    }
}

//----------------------------------------------------------------------------

#[start]
fn start(argc: int, argv: **u8) -> int {
    native::start(argc, argv, main)
}

fn main() {

    println!("GLFW version: {:s}", glfw::get_version_string());

    let (glfw, errors) = glfw::init().unwrap();
    glfw::fail_on_error(&errors);

    glfw.window_hint(glfw::ContextVersion(3, 2));
    glfw.window_hint(glfw::OpenglForwardCompat(true));
    glfw.window_hint(glfw::OpenglProfile(glfw::OpenGlCoreProfile));
    glfw.window_hint(glfw::Resizable(false));

    let (window, events) = glfw.create_window(512, 512,
                                              "MandelRust",
                                              glfw::Windowed)
        .expect({glfw::fail_on_error(&errors);
                 "Failed to create GLFW window."});

    glfw.make_context_current(Some(&window));
    window.set_key_polling(true);

    let mut win_ctrl = ~WindowController::new(&window);

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

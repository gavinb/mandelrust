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
// - cgmath
//
//============================================================================

#[license = "MIT"];

#[allow(deprecated_owned_vector)];
#[allow(dead_code)];

extern crate native;
extern crate num;

extern crate opengles;
extern crate glfw;
extern crate cgmath;

use opengles::gl2;

use cgmath::vector::Vec3;
use cgmath::aabb::Aabb3;

use std::comm::{Sender, Receiver, channel};
use std::vec_ng::Vec;
use std::io::File;
use std::path::Path;

use num::complex::{Cmplx, Complex64};

use std::num::{sin};

type Vec3f = Vec3<f32>;
type AABB3f = Aabb3<f32>;

//----------------------------------------------------------------------------

static vertex_shader_source: &'static str = "

#version 150

in vec2 position;
in vec3 color;

out vec3 Color;

void main()
{
    Color = color;
    gl_Position = vec4(position, 0.0, 1.0);
}
";

static fragment_shader_source: &'static str = "

#version 150

in vec3 Color;
out vec4 outColor;

void main()
{
    outColor = vec4(Color, 1.0);
}
";

//----------------------------------------------------------------------------

struct ErrorContext;
impl glfw::ErrorCallback for ErrorContext {
    fn call(&self, _: glfw::Error, description: ~str) {
        println!("GLFW Error: {:s}", description);
    }
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
    uni_color: gl2::GLint,
    buffer_width: uint,
    buffer_height: uint,
}

impl<'a> WindowController<'a> {
    fn new(window: &'a glfw::Window) -> WindowController<'a> {
        let mut wc = WindowController { window: window, 
                                        vao: ~[0],
                                        vbo: ~[0],
                                        vertices: ~[],
                                        vertex_shader: 0,
                                        fragment_shader: 0,
                                        shader_program: 0,
                                        uni_color: 0,
                                        buffer_width: 0,
                                        buffer_height: 0,
        };

        let (w, h) =  wc.window.get_framebuffer_size();
        wc.buffer_width = w as uint;
        wc.buffer_height = h as uint;

        gl2::viewport(0, 0, wc.buffer_width as i32, wc.buffer_height as i32);

        println!("Viewport: {} x {}", wc.buffer_width, wc.buffer_height);

        wc.vertices = ~[
            0.0,  0.5, 1.0, 0.0, 0.0,
            0.5, -0.5, 0.0, 1.0, 0.0,
            -0.5, -0.5, 0.0, 0.0, 1.0,
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
        gl2::vertex_attrib_pointer_f32(pos_attrib as gl2::GLuint, 2, false, 5*4, 0);

        let col_attrib = gl2::get_attrib_location(wc.shader_program, "color");
        gl2::enable_vertex_attrib_array(col_attrib as gl2::GLuint);
        gl2::vertex_attrib_pointer_f32(col_attrib as gl2::GLuint, 3, false, 5*4, 2*4);

        // Uniforms

        wc.uni_color = gl2::get_uniform_location(wc.shader_program, "triangleColor");

        let err = gl2::get_error();
        if err != 0 {
            fail!("attrib err = {:x}", err);
        }

        wc
    }

    // Rescale pixel coord (x,y) into cspace
    fn scale_coords(&self, x: uint, y: uint) -> (f32, f32) {
        let x0 = -2.5f32;
        let x1 =  1.0f32;
        let y0 = -1.0f32;
        let y1 =  1.0f32;

        let xx = (x as f32) / (self.buffer_width  as f32) * (x1-x0) + x0;
        let yy = (y as f32) / (self.buffer_height as f32) * (y1-y0) + y0;

        (xx, yy)
    }

    fn draw(&self) {

        // Clear

        gl2::clear_color(0.4, 0.4, 0.4, 1.0);
        gl2::clear(gl2::COLOR_BUFFER_BIT);

        // Animate

        let time = glfw::get_time();
        gl2::uniform_3f(self.uni_color, (sin(time * 4.0f32 as f64) as f32 + 1.0f32) / 2.0f32, 0.0f32, 0.0f32);

        // Draw!

        gl2::draw_arrays(gl2::TRIANGLES, 0, 3);

        let err = gl2::get_error();
        if err != 0 {
            println!("draw err = 0x{:x}", err);
        }

        self.window.swap_buffers();
    }

    fn uninit(&mut self) {

        // Cleanup

        gl2::delete_program(self.shader_program);
        gl2::delete_shader(self.fragment_shader);
        gl2::delete_shader(self.vertex_shader);

        gl2::delete_buffers(self.vbo);

        // @todo Missing from bindings
        //gl2::delete_vertex_arrays(vao);
    }

    fn handle_window_event(&self, window: &glfw::Window, (time, event): (f64, glfw::WindowEvent)) {
        match event {

            glfw::CloseEvent => println!("Time: {}, Window close requested.", time),

            glfw::KeyEvent(key, scancode, action, mods) => {
                println!("Time: {}, Key: {}, ScanCode: {}, Action: {}, Modifiers: [{}]", time, key, scancode, action, mods);
                match (key, action) {
                    (glfw::KeyEscape, glfw::Press) => window.set_should_close(true),
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

//----------------------------------------------------------------------------

struct MandelEngine {
    buffer_width: uint,
    buffer_height: uint,
    buffer: Vec<u8>,// RGBA
}

impl MandelEngine {

    fn new(w: uint, h: uint) -> MandelEngine {
        MandelEngine {
            buffer_width: w,
            buffer_height: h,
            buffer: Vec::with_capacity(w*h), // Monochrome, 8-bit
        }
    }

    // Rescale pixel coord (x,y) into cspace
    fn scale_coords(&self, x: uint, y: uint) -> (f32, f32) {
        let x0 = -2.5f32;
        let x1 =  1.0f32;
        let y0 = -1.0f32;
        let y1 =  1.0f32;

        let xx = (x as f32) / (self.buffer_width  as f32) * (x1-x0) + x0;
        let yy = (y as f32) / (self.buffer_height as f32) * (y1-y0) + y0;

        (xx, yy)
    }

    // Evalute entire region
    fn process(&mut self) {

        let max_iteration = 255;

        println!("+++ process in {} bytes", self.buffer.capacity());

        // Process each pixel
        for py in range(0, self.buffer_height) {
            for px in range(0, self.buffer_width) {

                // Project pixels into Mandelbrot domain
                let (x0, y0) = self.scale_coords(px, py);

                //println!("({},{}) -> ({}, {})", px, py, x0, y0);

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
                //color = palette[iteration];
                let color = iteration;
                let ofs = px+self.buffer_width*py;
//                *self.buffer.get_mut(ofs) = color;
                self.buffer.push(color);
            }
        }

        // Save
        let filename = "m1.pgm";
        let mut file = File::create(&Path::new(filename));
        file.write(bytes!("P5\n"));
        file.write_str(format!("{} {}\n255\n", self.buffer_width, self.buffer_height));
        file.write(self.buffer.slice(0, self.buffer.capacity()));
    }

    // Evaluate a single point
    fn mandel(&self, z: Complex64) -> uint {
        let maxiter: uint = 80;
        let mut c: Complex64 = z;
        for n in range(0, maxiter) {
            if c.norm() > 2.0 {
                return n;
            }
            c = c*c+z;
        }
        return maxiter;
    }
}

//----------------------------------------------------------------------------

fn start_engine(tx: &Sender<uint>) {
    let mut engine = MandelEngine::new(1920, 960);
    engine.process();
//    tx.send(42);
}

//----------------------------------------------------------------------------

#[start]
fn start(argc: int, argv: **u8) -> int {
    native::start(argc, argv, main)
}

fn main() {

    println!("GLFW version: {:s}", glfw::get_version_string());

    glfw::start(proc() {

        glfw::window_hint::context_version_major(3);
        glfw::window_hint::context_version_minor(2);
        glfw::window_hint::opengl_forward_compat(true);
        glfw::window_hint::opengl_profile(glfw::OpenGlCoreProfile);
        glfw::window_hint::resizable(false);

        let window = glfw::Window::create(640, 480,
                                          "MandelRust",
                                          glfw::Windowed)
            .expect("Failed to create GLFW window.");

        window.make_context_current();

        let win_ctrl = ~WindowController::new(&window);

        let (tx, rx): (Sender<uint>, Receiver<uint>) = channel();

        native::task::spawn( proc() {
            start_engine(&tx);
        });

//         let indata = rx.recv();
//         println!("indata {}", indata);

        while !window.should_close() {
            glfw::poll_events();
            for event in window.flush_events() {
                win_ctrl.handle_window_event(&window, event);
            }
            win_ctrl.draw();
        }
    });
}

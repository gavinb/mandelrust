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

extern crate native;
extern crate num;

extern crate opengles;
extern crate glfw;
extern crate cgmath;

use opengles::gl2;

use cgmath::vector::Vec3;
use cgmath::aabb::Aabb3;

use std::str;
use std::libc;
use num::complex::Complex64;
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

struct WindowController<'a> {
    window: &'a glfw::Window,
    vertices: ~[f32],
    vao: ~[gl2::GLuint],
    vbo: ~[gl2::GLuint],
    vertex_shader: gl2::GLuint,
    fragment_shader: gl2::GLuint,
    shader_program: gl2::GLuint,
    uni_color: gl2::GLint,
}

struct KeyController;

impl KeyController {
    fn call(&self, window: &glfw::Window, key: glfw::Key, _: libc::c_int, action: glfw::Action, _: glfw::Modifiers) {
        if action == glfw::Press && key == glfw::KeyEscape {
            window.set_should_close(true);
        }
    }
}

struct MandelEngine {
    re0: f32,
    re1: f32,
    im0: f32,
    im1: f32,
    delta: f32,
}

impl MandelEngine {

    fn new() -> MandelEngine {
        MandelEngine { re0: -2.0, re1: 2.0, im0: -2.0, im1: 2.0, delta: 1.0 }
    }

    // Evalute entire region
    fn process() {
    }

    // Evaluate a single point
    fn mandel(z: Complex64) -> int {
        let maxiter: int = 80;
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
        };

        let (width, height) =  wc.window.get_framebuffer_size();

        gl2::viewport(0, 0, width as i32, height as i32);

        println!("Viewport: {} x {}", width, height);

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

        gl2::shader_source(wc.vertex_shader, ~[vertex_shader_source.to_owned().as_bytes()]);

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

        gl2::shader_source(wc.fragment_shader, ~[fragment_shader_source.to_owned().as_bytes()]);

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

    fn draw(&self) {

        // Clear

        gl2::clear_color(0.4, 0.4, 0.4, 1.0);
        gl2::clear(gl2::COLOR_BUFFER_BIT);

        // Animate

        let time = glfw::get_time();
//        gl2::uniform_3f(wc.uni_color, (sin(time * 4.0) + 1.0) / 2.0, 0.0, 0.0);

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

        while !window.should_close() {
            glfw::poll_events();
            for event in window.flush_events() {
                win_ctrl.handle_window_event(&window, event);
            }
        }
    });
}

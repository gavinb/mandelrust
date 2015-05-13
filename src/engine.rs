//============================================================================
//
// A simple Mandelbrot image generator in Rust
//
// Generator Engine backend
//
// Copyright (c) 2014 Gavin Baker <gavinb@antonym.org>
// Published under the MIT license
//
//============================================================================

use std::comm::{Sender, Receiver};
use std::vec::Vec;
use std::num::Float;

use protocol::{RenderType, EngineStatus, EngineCommand};

use protocol;

static PREVIEW_WIDTH: i32 = 256;
static PREVIEW_HEIGHT: i32 = 256;

//----------------------------------------------------------------------------

type RGB8 = (u8, u8, u8);

pub struct MandelEngine {
    buffer_width: uint,
    buffer_height: uint,
    palette: Vec<RGB8>,
    re0: f32,
    re1: f32,
    im0: f32,
    im1: f32,
}

impl MandelEngine {

    pub fn new(w: uint, h: uint) -> MandelEngine {

        // Init palette using hue sweep in HSV colour space
        let mut p: Vec<RGB8> = Vec::with_capacity(720);
        let s = 1.0f32; // saturation
        let v = 1.0f32; // value
        let c = v * s; // chroma
        for h in range(0, 720u) { // hue
            let hp = h as f32/60.0;
            let x = c * (1.0-(hp % 2.0 - 1.0).abs());
            let (r,g,b) = if 0.0 <= hp && hp < 1.0 {
                (c, x, 0.0)
            } else if 1.0 <= hp && hp < 2.0 {
                (x, c, 0.0)
            } else if 2.0 <= hp && hp < 3.0 {
                (0.0, c, x)
            } else if 3.0 <= hp && hp < 4.0 {
                (0.0, x, c)
            } else if 4.0 <= hp && hp < 5.0 {
                (x, 0.0, c)
            } else if 5.0 <= hp && hp < 6.0 {
                (c, 0.0, x)
            } else {
                (0.0, 0.0, 0.0)
            };
            let m = v-c;
            p.push((((r+m)*255.0) as u8, ((g+m)*255.0) as u8, ((b+m)*255.0) as u8));
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

    pub fn serve(&mut self, cmd_chan: &Receiver<EngineCommand>, progress_chan: &Sender<EngineStatus>) {
        let mut running = true;
        while running {
            // pan/zoom by 10% of width
            let delta_r = ((self.re1 - self.re0)*0.1f32).abs();
            let delta_i = ((self.im1 - self.im0)*0.1f32).abs();
            println!("delta r,i {},{}", delta_r, delta_i);

            let cmd = cmd_chan.recv();
            println!("engine: command {}", cmd);
            match cmd {
                EngineCommand::UpdateRegion(re0, re1, im0, im1) => {
                    self.re0 = re0; self.re1 = re1; self.im0 = im0; self.im1 = im1;
                },
                EngineCommand::ZoomIn => {
                    self.re0 += delta_r;
                    self.re1 -= delta_r;
                    self.im0 += delta_i;
                    self.im1 -= delta_i;
                },
                EngineCommand::ZoomOut => {
                    self.re0 -= delta_r;
                    self.re1 += delta_r;
                    self.im0 -= delta_i;
                    self.im1 += delta_i;
                },
                EngineCommand::PanLeft => {
                    self.re0 -= delta_r;
                    self.re1 -= delta_r;
                },
                EngineCommand::PanRight => {
                    self.re0 += delta_r;
                    self.re1 += delta_r;
                },
                EngineCommand::PanUp => {
                    self.im0 += delta_i;
                    self.im1 += delta_i;
                },
                EngineCommand::PanDown => {
                    self.im0 -= delta_i;
                    self.im1 -= delta_i;
                },
                EngineCommand::Render(typ) => self.process(typ, progress_chan),
                EngineCommand::Shutdown => running = false,
            }
        }

        println!("engine: shutdown");
    }

    // Evalute entire region
    fn process(&mut self, typ: RenderType, progress_chan: &Sender<EngineStatus>) {

        let (width, height) = match typ {
            RenderType::PreviewRender => (PREVIEW_WIDTH as uint, PREVIEW_HEIGHT as uint),
            RenderType::FullRender => (self.buffer_width, self.buffer_height),
        };

        let mut img: Vec<u8> = Vec::with_capacity(width*height*3);

        let max_iteration = 500;

        println!("+++ process {}x{} RGB8 in {} bytes", width, height, img.capacity());
        println!("            re: {}..{} im: {}..{}", self.re0, self.re1, self.im0, self.im1);

        progress_chan.send(EngineStatus::Startup);

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
                let &(r,g, b) = color.unwrap();

                img.push(r);
                img.push(g);
                img.push(b);
            }
            if py % 100 == 0 {
                progress_chan.send(EngineStatus::Processing(py));
            }
        }

        progress_chan.send(EngineStatus::RenderComplete(typ, img));
    }
}

//----------------------------------------------------------------------------

//============================================================================
//
// A simple Mandelbrot image generator in Rust
//
// Command-line Interface
//
// Copyright (c) 2014 Gavin Baker <gavinb@antonym.org>
// Published under the MIT license
//
//============================================================================

use std::comm::{Sender, Receiver, Data, channel};
use std::vec::Vec;
use std::io::File;
use std::path::Path;

// @note Why should we have to import the enum _and_ all the variants?
use protocol::{PreviewRender, FullRender};
use protocol::{EngineStatus, Startup, Processing, RenderComplete, Error};
use protocol::{EngineCommand, Render, Shutdown};
use protocol::{PREVIEW_WIDTH, PREVIEW_HEIGHT};

use engine::MandelEngine;

// @note Why does the mod come after the use that refers to it?
mod engine;
mod protocol;

//----------------------------------------------------------------------------

struct CommandLine {
    width: uint,
    height: uint,
    chan_cli_to_engine: Option<Sender<EngineCommand>>,
    chan_cli_from_engine: Option<Receiver<EngineStatus>>,
    chan_engine_to_cli: Option<Sender<EngineStatus>>,
    chan_engine_from_cli: Option<Receiver<EngineCommand>>,
    image: Option<Vec<u8>>,
}

impl CommandLine {

    pub fn new(w: uint, h: uint) -> CommandLine {
        let (chan_cli_to_engine, chan_engine_from_cli) = channel();
        let (chan_engine_to_cli, chan_cli_from_engine) = channel();
        CommandLine {
            width: w,
            height: h,
            chan_cli_to_engine: Some(chan_cli_to_engine),
            chan_cli_from_engine: Some(chan_cli_from_engine),
            chan_engine_to_cli: Some(chan_engine_to_cli),
            chan_engine_from_cli: Some(chan_engine_from_cli),
            image: None,
        }
    }

    pub fn start_engine(&mut self) {

        let progress_ch = self.chan_engine_to_cli.take().expect("no engine_to_cli chan");
        let cmd_ch = self.chan_engine_from_cli.take().expect("no engine_from_cli chan");

        let (w,h) = (self.width, self.height);

        native::task::spawn( proc() {

            let mut engine = MandelEngine::new(w, h);
            engine.serve(&cmd_ch, &progress_ch);
        });

        let cmd_ch = self.chan_cli_to_engine.get_ref();
        cmd_ch.send(Render(FullRender));
    }

    pub fn stop_engine(&mut self) {
        let cmd_ch = self.chan_cli_to_engine.get_ref();
        cmd_ch.send(Shutdown);
    }

    pub fn handle_update(&mut self) -> bool {
        match self.chan_cli_from_engine {
            Some(ref ch) => {
                let status_msg = ch.try_recv();
                match status_msg {
                    Data(status) =>
                        match status {
                            Startup => {
                                println!("Startup...");
                                false
                            },
                            Processing(progress) => {
                                println!("Processing {}", progress);
                                false
                            },
                            RenderComplete(typ, img) => {
                                println!("Render Complete!");
                                self.image = Some(img);
                                match typ {
                                    FullRender => {
                                        println!("fullRender {} {}", self.width, self.height);
                                    },
                                    PreviewRender => {
                                        println!("Preview {} {}", PREVIEW_WIDTH, PREVIEW_HEIGHT);
                                    },
                                };
                                true
                            },
                            Error(code) => {
                                println!("Error {}", code);
                                false
                            },
                        },
                    _ => false,
                }
            },
            None => false,
        }
    }

    pub fn save_as_ppm(&self, filename: &str) {
        match self.image {
            Some(ref img) => {
                println!("Saving {}", filename);
                let mut file = File::create(&Path::new(filename));
                file.write(bytes!("P6\n"));
                file.write_str(format!("{} {}\n255\n", self.width, self.height));
                file.write(img.slice(0, img.capacity()));
            },
            None => (),
        }
    }
}

fn main() {

    let mut cli = CommandLine::new(640, 640);

    cli.start_engine();

    loop {
        if cli.handle_update() == true {
            cli.save_as_ppm("test.ppm");
            cli.stop_engine();
            break;
        }
    }
}

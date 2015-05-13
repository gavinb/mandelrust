//============================================================================
//
// A simple Mandelbrot image generator in Rust
//
// Protocol for communicating with Engine task
//
// Copyright (c) 2014 Gavin Baker <gavinb@antonym.org>
// Published under the MIT license
//
//============================================================================

use std::vec::Vec;

//----------------------------------------------------------------------------

#[derive(Debug)]
pub enum RenderType {
    PreviewRender,
    FullRender,
}

// @todo Why does the compiler complain when this is used in the importing module?
// "warning: code is never used: `PREVIEW_WIDTH`, #[warn(dead_code)] on by default"
pub static PREVIEW_WIDTH: i32 = 256;
pub static PREVIEW_HEIGHT: i32 = 256;

//----------------------------------------------------------------------------

#[derive(Debug)]
pub enum EngineStatus {
    Startup,
    Processing(u32),
    RenderComplete(RenderType, Vec<u8>),
    Error(u32)
}

//----------------------------------------------------------------------------

#[derive(Debug)]
pub enum EngineCommand {
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

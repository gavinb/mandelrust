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

#[deriving(Show)]
pub enum RenderType {
    PreviewRender,
    FullRender,
}

//----------------------------------------------------------------------------

#[deriving(Show)]
pub enum EngineStatus {
    Startup,
    Processing(uint),
    RenderComplete(RenderType, Vec<u8>),
    Error(uint)
}

//----------------------------------------------------------------------------

#[deriving(Show)]
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

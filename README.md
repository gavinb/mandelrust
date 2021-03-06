==========
Mandelrust
==========

- Author: Gavin Baker
- Email: gavinb@antonym.org
- Date: August 2015
- Version: 0.5
- Web: https://github.com/gavinb/mandelrust

Introduction
============

This application provides an interactive viewer of the [Mandelbrot
Set](http://en.wikipedia.org/Mandelbrot%20set).  It was written as an
exercise in Rust graphics and tasks, and to explore Mandelbrot rendering.

License
=======

This project is licensed under the MIT License - see the LICENSE file for
full details.  Basically this is the simplest, most permissive license I
could find.

Obtaining the Source
====================

The latest source is always available from the Github project page:

    https://github.com/gavinb/mandelrust

To download, simply run:

    $ git clone https://github.com/gavinb/mandelrust.git

Building
========

This code was written to work with Rust 1.8 and build with contemporary
versions of 3rd party packages from `crates.io`.

The following libraries are required:

 - *glium*: Glium library, for managing the OpenGL window and I/O events
 - *glutin*: Glutin provides cross-platform OpenGL context management

This project uses Cargo, so simply running:

    cargo build

should be sufficient to build and install the dependencies.

Controls
========

The keyboard can be used to control the app.  Once the user presses a key to
enter interactive mode, the set is rendered quickly at a low resolution.
Once the user is happy with the displayed view, she may press Space to
render the set at full resolution (which can take a while).

     | Key              | Function
     +------------------+--------------------------------------
     | Arrow Keys       | Pan left/right/up/down around the set
     | +/- Keys         | Zoom in/out about the centre
     | Space            | Redraw in full resolution

Future
======

I am considering using this project as a basis for a series of Rust
tutorials, starting with the simplest possible renderer and building up to a
full interactive GUI.

Obvious improvements include interactive mouse zoom/pan controls, palette
controls, and other set functions (eg. Julia Sets).

In the meantime, If you have any patches, please send them along via Github.

:: Gavin Baker

-- Melbourne, Summer 2014

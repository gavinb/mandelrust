extern crate gl_generator;
extern crate khronos_api;

use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap().into_string().unwrap();
    let dest = Path::new(&out_dir);

    let mut file = File::create(&dest.join("gl_bindings.rs")).unwrap();

    // This generates bindsings for OpenGL ES v3.1
    gl_generator::generate_bindings(gl_generator::GlobalGenerator,     // generator
                                    gl_generator::registry::Ns::Gles2, // namespace
                                    gl_generator::Fallbacks::All,      // fallbacks
                                    khronos_api::GL_XML,               // source
                                    vec![],          // extensions
                                    "3.1",  // version
                                    "core",  // profile
                                    &mut file // dest
                                   ).unwrap();
}

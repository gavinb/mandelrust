
RUSTC=rustc
RUST_PATH=~/src/cgmath-rs/lib:~/src/rust-opengles:~/src/glfw-rs/lib
RUST_LIBS=-L ~/src/cgmath-rs/lib -L ~/src/rust-opengles -L ~/src/glfw-rs/lib
RUST_FLAGS=$(RUST_LIBS)

SOURCES=mandelrust.rs

# rustc -L ~/src/cgmath-rs/lib -L ~/src/rust-opengles -L ~/src/glfw-rs/lib mandelrust.rs

%:%.rs
	$(RUSTC) $(RUST_FLAGS) $<

all:		mandelrust

mandelrust:	mandelrust.rs

clean:		#
		@rm -f mandelrust

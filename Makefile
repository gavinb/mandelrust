
RUSTC=rustc
RUST_LIBS=-L ~/src/rust-opengles -L ~/src/glfw-rs/lib
RUST_FLAGS=$(RUST_LIBS) -g

SOURCES=mandelrust.rs

%:%.rs
	$(RUSTC) $(RUST_FLAGS) $<

all:		mandelrust

mandelrust:	mandelrust.rs

clean:		#
		@rm -f mandelrust mandelrust.dSYM

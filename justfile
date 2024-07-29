BUILD_DIR := "builddir"
CAPNP_DIR := "generated"

default: build

generate_capnp:
  mkdir -p {{CAPNP_DIR}}
  capnp compile -oc++:{{CAPNP_DIR}} program.capnp

configure: generate_capnp
  meson setup --buildtype=debug {{BUILD_DIR}}

build: configure
  meson compile -C {{BUILD_DIR}}

test: build
  meson test -C {{BUILD_DIR}}

release:
  meson setup --buildtype=release {{BUILD_DIR}}
  meson compile -C {{BUILD_DIR}}

format:
  clang-format -i ./src/**
  clang-format -i ./include/**

clean:
  rm -rf {{BUILD_DIR}}

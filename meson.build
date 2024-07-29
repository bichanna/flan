project('impala', 'cpp',
  version: '0.0.0',
  license: 'MIT',
  default_options: [
    'warning_level=3',
    'werror=true',
    'cpp_std=c++17'
  ]
)

cpp = meson.get_compiler('cpp')

kj_dep = cpp.find_library('kj')
capnp_dep = cpp.find_library('capnp')

impala_include = include_directories('include')

src = [
  'src/impala.cpp',
]

executable('impala',
  sources: [src, 'generated/program.capnp.c++'],
  include_directories: [impala_include, 'generated'],
  dependencies: [kj_dep, capnp_dep]
)

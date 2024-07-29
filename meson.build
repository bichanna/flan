project('impala', 'cpp',
  version: '0.0.0',
  license: 'MIT',
  default_options: [
    'warning_level=3',
    'werror=true',
    'cpp_std=c++17'
  ]
)

impala_include = include_directories('include')

src = [
  'src/impala.cpp',
]

executable('impala',
  sources: [src, 'generated/program.capnp.c++'],
  include_directories: [impala_include, 'generated'],
)

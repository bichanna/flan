project('flan', 'cpp',
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
  'src/flan.cpp',
  'src/gc.cpp',
  'src/vm.cpp',
]

executable('flan',
  sources: src,
  include_directories: impala_include,
)

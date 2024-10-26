project('flan', 'c',
  version: '0.0.0',
  license: 'MIT',
  default_options: [
    'warning_level=3',
    'werror=true',
    'c_std=c99'
  ]
)

src = [
  'src/utf8.h',
  'src/flan.h',
  'src/flan.c',
  'src/value.h',
  'src/value.c',
  'src/stack.h',
  'src/stack.c',
  'src/gc.h',
  'src/gc.c',
  'src/hashmap.h',
  'src/hashmap.c',
  'src/vm.h',
  'src/vm.c',
]

executable('flan',
  sources: src,
)

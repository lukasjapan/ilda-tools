project('ilda-tools', 'cpp', default_options : ['cpp_std=c++11'])
cpp = meson.get_compiler('cpp')

src = [
    'ilda-player.cpp',
    'src/fltk_output.cpp',
    'src/ilda_istream.cpp',
    'src/ilda_repeater.cpp'
]
deps = [
    cpp.find_library('fltk'),
    cpp.find_library('boost_program_options')
]
executable('ilda-player', src, dependencies: deps)
project('ilda-tools', 'cpp', default_options : ['cpp_std=c++11'])
cpp = meson.get_compiler('cpp')

boost_program_options = cpp.find_library('boost_program_options')
fltk = cpp.find_library('fltk')

src = [
    'ilda-player.cpp',
    'src/fltk_output.cpp',
    'src/ilda_istream.cpp',
    'src/ilda_repeater.cpp'
]
executable('ilda-player', src, dependencies: [ boost_program_options, fltk ])

src  = [
    'ilda-wav.cpp',
    'src/ilda_istream.cpp',
    'src/ilda_wav.cpp'
]
executable('ilda-wav', src, dependencies: [ boost_program_options ])
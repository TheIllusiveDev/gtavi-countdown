project(
  'gtavi-countdown',
  # C compiles the resource bundle that glib-compile-resources generates.
  'c',
  'cpp',
  version: '1.0.0',
  license: 'Unlicense',
  meson_version: '>= 1.3.0',
  default_options: [
    'cpp_std=c++23',
    'warning_level=3',
    'buildtype=debugoptimized',
  ],
)

app_id = 'app.gtavi.countdown'
pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()

# ------------------------------------------------------------------ core
# Release data, time zones, countdown maths, iCalendar, settings, themes and
# the SNTP client. No UI dependencies.

json = dependency('nlohmann_json', version: '>= 3.11')

core_lib = static_library(
  'gtavi-core',
  files(
    'src/core/calendar_file.cpp',
    'src/core/countdown.cpp',
    'src/core/ics.cpp',
    'src/core/release.cpp',
    'src/core/settings.cpp',
    'src/core/sntp.cpp',
    'src/core/themes.cpp',
    'src/core/wording.cpp',
    'src/core/zones.cpp',
  ),
  dependencies: [json],
)

core_dep = declare_dependency(
  link_with: core_lib,
  include_directories: include_directories('src/core'),
)

# ------------------------------------------------------------------- app

if get_option('app')
  gtkmm = dependency('gtkmm-4.0', version: '>= 4.14')
  gtk = dependency('gtk4', version: '>= 4.16')
  epoxy = dependency('epoxy')
  threads = dependency('threads')

  app_deps = [core_dep, json, gtkmm, gtk, epoxy, threads]
  app_args = [
    '-DAPP_ID="@0@"'.format(app_id),
    '-DAPP_VERSION="@0@"'.format(meson.project_version()),
    '-DPKGDATADIR="@0@"'.format(pkgdatadir),
    '-DSOURCE_DATADIR="@0@"'.format(meson.current_source_dir() / 'data'),
  ]

  # Always on top has no GTK 4 API; on X11 it is requested from the window
  # manager directly. Wayland offers no way to ask for it.
  x11 = dependency('x11', required: get_option('x11'))
  gtk_x11 = dependency('gtk4-x11', required: get_option('x11'))
  if x11.found() and gtk_x11.found()
    app_deps += [x11, gtk_x11]
    app_args += ['-DHAVE_X11=1']
  endif

  gnome = import('gnome')
  resources = gnome.compile_resources(
    'resources',
    'data/gtavi-countdown.gresource.xml',
    source_dir: 'data',
    c_name: 'gtavi_resources',
  )
  # The generated bundle is one long string literal; that is fine for GCC and Clang.
  resources_lib = static_library(
    'gtavi-resources',
    resources,
    c_args: ['-Wno-overlength-strings'],
    dependencies: [gtk],
  )

  executable(
    'gtavi-countdown',
    files(
      'src/app/app_paths.cpp',
      'src/app/backdrop.cpp',
      'src/app/background.cpp',
      'src/app/clock.cpp',
      'src/app/countdown_view.cpp',
      'src/app/icon.cpp',
      'src/app/main.cpp',
      'src/app/main_window.cpp',
      'src/app/platform.cpp',
      'src/app/settings_panel.cpp',
      'src/app/stats_view.cpp',
      'src/app/timeline_view.cpp',
      'src/app/window_state.cpp',
    ),
    dependencies: app_deps,
    link_whole: resources_lib,
    cpp_args: app_args,
    install: true,
  )

  install_data(
    'data/fonts/archivo-latin-standard-normal.ttf',
    'data/fonts/archivo-latin-ext-standard-normal.ttf',
    'data/fonts/jetbrains-mono-latin-wght-normal.ttf',
    'data/fonts/LICENSE-archivo.txt',
    'data/fonts/LICENSE-jetbrains-mono.txt',
    install_dir: pkgdatadir / 'fonts',
  )
  install_data(
    'data/@0@.desktop'.format(app_id),
    install_dir: get_option('datadir') / 'applications',
  )
  install_data(
    'data/icons/@0@.svg'.format(app_id),
    install_dir: get_option('datadir') / 'icons' / 'hicolor' / 'scalable' / 'apps',
  )
endif

# ----------------------------------------------------------------- tests

gtest = dependency('gtest', main: true, required: get_option('tests'))
if gtest.found()
  tests = executable(
    'gtavi-tests',
    files(
      'tests/calendar_file_test.cpp',
      'tests/countdown_test.cpp',
      'tests/ics_test.cpp',
      'tests/settings_test.cpp',
      'tests/sntp_test.cpp',
      'tests/zones_test.cpp',
    ),
    dependencies: [core_dep, gtest, dependency('threads')],
  )
  test('core', tests, protocol: 'gtest', timeout: 60)
endif

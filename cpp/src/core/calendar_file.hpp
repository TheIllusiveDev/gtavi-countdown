#pragma once
// Validation for calendar files the app writes. Kept from the web-view
// versions, where the name and contents came from page script: saving must
// not be usable to write an arbitrary file to an arbitrary place.

#include <cstddef>
#include <stdexcept>
#include <string>
#include <string_view>

namespace gtavi::calendar_file {

// Largest calendar file the app will write. A single event is under 1 KiB.
inline constexpr std::size_t max_bytes = 64 * 1024;

// Checks a calendar export and returns the file name to save it under. The
// name must be a bare .ics file name with no directory components, and the
// contents an iCalendar object of a sane size.
// Throws std::invalid_argument when either is not acceptable.
std::string validate(std::string_view file_name, std::string_view contents);

}  // namespace gtavi::calendar_file

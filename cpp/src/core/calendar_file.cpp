#include "calendar_file.hpp"

#include <algorithm>
#include <cctype>

namespace gtavi::calendar_file {

namespace {
// Same whitespace set as .NET's string.Trim for ASCII input.
bool is_space(char c) { return c == ' ' || (c >= '\t' && c <= '\r'); }

std::string_view trim_end(std::string_view s) {
    while (!s.empty() && is_space(s.back())) s.remove_suffix(1);
    return s;
}

std::string_view trim(std::string_view s) {
    while (!s.empty() && is_space(s.front())) s.remove_prefix(1);
    return trim_end(s);
}

bool allowed(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '-' || c == '_' ||
           c == '.';
}

bool ends_with_ics(std::string_view name) {
    if (name.size() < 4) return false;
    auto tail = name.substr(name.size() - 4);
    return std::ranges::equal(tail, std::string_view{".ics"},
                              [](char a, char b) { return std::tolower(static_cast<unsigned char>(a)) == b; });
}
}  // namespace

std::string validate(std::string_view file_name, std::string_view contents) {
    auto name = trim(file_name);
    if (name.empty() || name.size() > 64 || name.front() == '.' || !std::ranges::all_of(name, allowed) ||
        !ends_with_ics(name)) {
        throw std::invalid_argument("The calendar file name is not valid.");
    }
    if (contents.size() > max_bytes) throw std::invalid_argument("The calendar file is too large.");
    auto body = trim_end(contents);
    if (!body.starts_with("BEGIN:VCALENDAR") || !body.ends_with("END:VCALENDAR")) {
        throw std::invalid_argument("The data is not a calendar file.");
    }
    return std::string{name};
}

}  // namespace gtavi::calendar_file

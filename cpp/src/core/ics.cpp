#include "ics.hpp"

#include <chrono>
#include <format>

#include "zones.hpp"

namespace gtavi::ics {

std::string date(std::int64_t utc_ms) {
    auto w = zones::wall_time_at(utc_ms, "UTC");
    return std::format("{:04}{:02}{:02}T{:02}{:02}{:02}Z", w.year, w.month, w.day, w.hour, w.minute, w.second);
}

std::string escape(std::string_view text) {
    std::string out;
    out.reserve(text.size());
    for (std::size_t i = 0; i < text.size(); ++i) {
        char c = text[i];
        switch (c) {
            case '\\': out += "\\\\"; break;
            case ';': out += "\\;"; break;
            case ',': out += "\\,"; break;
            case '\r':
                if (i + 1 < text.size() && text[i + 1] == '\n') {
                    out += "\\n";
                    ++i;
                } else {
                    out += c;
                }
                break;
            case '\n': out += "\\n"; break;
            default: out += c;
        }
    }
    return out;
}

namespace {
// Length of the UTF-8 sequence that starts with `lead`. Stray continuation
// bytes count as one so malformed input still makes progress.
std::size_t sequence_length(unsigned char lead) {
    if (lead >= 0xF0) return 4;
    if (lead >= 0xE0) return 3;
    if (lead >= 0xC0) return 2;
    return 1;
}
}  // namespace

std::string fold(std::string_view line) {
    if (line.size() <= 75) return std::string{line};
    std::string out;
    std::size_t current = 0;
    // The first line may hold 75 octets; continuation lines start with a
    // space, which leaves 74 octets for content.
    std::size_t limit = 75;
    for (std::size_t i = 0; i < line.size();) {
        auto size = std::min(sequence_length(static_cast<unsigned char>(line[i])), line.size() - i);
        if (current + size > limit) {
            out += "\r\n ";
            current = 0;
            limit = 74;
        }
        out.append(line.substr(i, size));
        current += size;
        i += size;
    }
    return out;
}

std::string build_calendar(const CalendarEvent& e) {
    using namespace std::chrono;
    auto stamp = e.stamp.value_or(duration_cast<milliseconds>(system_clock::now().time_since_epoch()).count());
    std::vector<std::string> lines{
        "BEGIN:VCALENDAR",
        "VERSION:2.0",
        "PRODID:-//gtavi-countdown//Release Countdown//EN",
        "CALSCALE:GREGORIAN",
        "METHOD:PUBLISH",
        "BEGIN:VEVENT",
        "UID:" + e.uid,
        "DTSTAMP:" + date(stamp),
        "DTSTART:" + date(e.start),
        "DTEND:" + date(e.start + e.duration_minutes * 60'000LL),
        "SUMMARY:" + escape(e.summary),
        "DESCRIPTION:" + escape(e.description),
        "TRANSP:TRANSPARENT",
    };
    for (int minutes : e.reminders) {
        lines.emplace_back("BEGIN:VALARM");
        lines.emplace_back("ACTION:DISPLAY");
        lines.push_back("DESCRIPTION:" + escape(e.summary));
        lines.push_back(std::format("TRIGGER:-PT{}M", minutes));
        lines.emplace_back("END:VALARM");
    }
    lines.emplace_back("END:VEVENT");
    lines.emplace_back("END:VCALENDAR");
    std::string out;
    for (const auto& line : lines) out += fold(line) + "\r\n";
    return out;
}

}  // namespace gtavi::ics

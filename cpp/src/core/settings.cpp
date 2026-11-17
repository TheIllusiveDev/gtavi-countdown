#include "settings.hpp"

#include <array>

#include <nlohmann/json.hpp>

#include "zones.hpp"

namespace gtavi::settings {

namespace {

constexpr std::array<EffectLevel, 3> effect_levels{EffectLevel::automatic, EffectLevel::low, EffectLevel::off};

template <typename T, std::size_t N>
std::optional<T> parse_enum(const nlohmann::json& node, const std::array<T, N>& candidates) {
    if (!node.is_string()) return std::nullopt;
    const auto& text = node.get_ref<const std::string&>();
    for (auto candidate : candidates)
        if (key(candidate) == text) return candidate;
    return std::nullopt;
}

void read_bool(const nlohmann::json& raw, const char* name, bool& field) {
    if (auto it = raw.find(name); it != raw.end() && it->is_boolean()) field = it->get<bool>();
}

}  // namespace

std::string_view key(Theme theme) {
    switch (theme) {
        case Theme::midnight: return "midnight";
        case Theme::dawn: return "dawn";
        case Theme::dusk: break;
    }
    return "dusk";
}

std::string_view key(EffectLevel level) {
    switch (level) {
        case EffectLevel::low: return "low";
        case EffectLevel::off: return "off";
        case EffectLevel::automatic: break;
    }
    return "auto";
}

Settings sanitize(std::string_view json) {
    Settings result;
    auto raw = nlohmann::json::parse(json, nullptr, /*allow_exceptions=*/false);
    if (!raw.is_object()) return result;

    if (auto it = raw.find("timeZone"); it != raw.end() && it->is_string()) {
        const auto& zone = it->get_ref<const std::string&>();
        if (zones::is_valid_time_zone(zone)) result.time_zone = zone;
    }
    read_bool(raw, "showMilliseconds", result.show_milliseconds);
    if (auto it = raw.find("theme"); it != raw.end())
        if (auto theme = parse_enum(*it, themes::all)) result.theme = *theme;
    if (auto it = raw.find("effects"); it != raw.end())
        if (auto effects = parse_enum(*it, effect_levels)) result.effects = *effects;
    read_bool(raw, "notify", result.notify);
    read_bool(raw, "networkTime", result.network_time);
    read_bool(raw, "alwaysOnTop", result.always_on_top);
    return result;
}

std::string serialize(const Settings& data) {
    nlohmann::ordered_json out;
    out["timeZone"] = data.time_zone ? nlohmann::ordered_json(*data.time_zone) : nlohmann::ordered_json(nullptr);
    out["showMilliseconds"] = data.show_milliseconds;
    out["theme"] = key(data.theme);
    out["effects"] = key(data.effects);
    out["notify"] = data.notify;
    out["networkTime"] = data.network_time;
    out["alwaysOnTop"] = data.always_on_top;
    return out.dump();
}

Theme next(Theme theme) {
    for (std::size_t i = 0; i < themes::all.size(); ++i)
        if (themes::all[i] == theme) return themes::all[(i + 1) % themes::all.size()];
    return themes::all[0];
}

}  // namespace gtavi::settings

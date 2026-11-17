#include "settings_panel.hpp"

#include <gtk/gtk.h>
#include <gtkmm/cssprovider.h>
#include <gtkmm/gestureclick.h>
#include <gtkmm/grid.h>
#include <gtkmm/stylecontext.h>

#include <algorithm>
#include <cmath>
#include <format>
#include <sstream>

#include "countdown.hpp"
#include "icon.hpp"
#include "themes.hpp"
#include "zones.hpp"

namespace gtavi {

namespace {

struct ZoneEntry {
    std::string id;
    std::string city;
    std::string region;
    std::int64_t offset;
    std::string offset_text;
    std::string haystack;  // Lower-cased text the filter matches against.
};

std::string lower(std::string text) {
    std::ranges::transform(text, text.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return text;
}

// Offsets are evaluated at the release itself, not today, so that zones which
// change their clocks before then are listed with the right value.
ZoneEntry entry(const std::string& id) {
    static const auto reference = countdown::release_instant("UTC");
    auto offset = zones::offset_at(reference, id);
    auto offset_text = zones::format_offset(offset);
    auto city = zones::zone_city(id);
    auto region = zones::zone_region(id);
    auto haystack = lower(id + " " + city + " " + region + " " + offset_text);
    std::ranges::replace(haystack, '_', ' ');
    return {id, city, region, offset, offset_text, haystack};
}

// Built once. The zone furthest east unlocks first, so the list doubles as
// the order in which the release moves around the planet.
const std::vector<ZoneEntry>& all_zones() {
    static const auto zones = [] {
        std::vector<ZoneEntry> result;
        for (const auto& id : zones::list_time_zones()) result.push_back(entry(id));
        std::ranges::stable_sort(result, [](const ZoneEntry& a, const ZoneEntry& b) {
            if (a.offset != b.offset) return a.offset > b.offset;
            return lower(a.city) < lower(b.city);
        });
        return result;
    }();
    return zones;
}

std::vector<std::string> words(const std::string& text) {
    std::vector<std::string> result;
    std::istringstream stream(text);
    for (std::string word; stream >> word;) result.push_back(word);
    return result;
}

Gtk::Label& label(const std::string& text, const char* css_class = nullptr) {
    auto& result = *Gtk::make_managed<Gtk::Label>(text);
    result.set_xalign(0);
    if (css_class != nullptr) result.add_css_class(css_class);
    return result;
}

Gtk::Label& note(const std::string& text) {
    auto& result = label(text, "note");
    result.set_wrap(true);
    result.set_wrap_mode(Pango::WrapMode::WORD_CHAR);
    return result;
}

std::string upper(std::string text) {
    std::ranges::transform(text, text.begin(), [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
    return text;
}

std::string hex(Rgb color) {
    auto byte = [](float v) { return static_cast<int>(std::lround(std::clamp(v, 0.0f, 1.0f) * 255)); };
    return std::format("#{:02x}{:02x}{:02x}", byte(color.r), byte(color.g), byte(color.b));
}

// The swatches are gradients, which only CSS can give a plain box.
void install_swatch_styles() {
    static bool installed = false;
    if (installed) return;
    installed = true;
    std::string css;
    for (auto theme : themes::all) {
        const auto& swatch = themes::info(theme).swatch;
        css += std::format(".swatch-{} {{ background-image: linear-gradient(180deg, {}, {} 55%, {}); }}\n",
                           settings::key(theme), hex(swatch[0]), hex(swatch[1]), hex(swatch[2]));
    }
    auto provider = Gtk::CssProvider::create();
    provider->load_from_string(css);
    Gtk::StyleProvider::add_provider_for_display(Gdk::Display::get_default(), provider,
                                                 GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
}

void accessible_label(Gtk::Widget& widget, const std::string& text) {
    gtk_accessible_update_property(GTK_ACCESSIBLE(widget.gobj()), GTK_ACCESSIBLE_PROPERTY_LABEL, text.c_str(), -1);
}

constexpr std::array<EffectLevel, 3> effect_levels{EffectLevel::automatic, EffectLevel::low, EffectLevel::off};

}  // namespace

// ------------------------------------------------------------ zone picker

ZonePicker::ZonePicker(std::function<void(std::optional<std::string>)> on_select)
    : Gtk::Box(Gtk::Orientation::VERTICAL, 10), on_select_(std::move(on_select)) {
    search_.set_placeholder_text("Search city, region or offset");
    accessible_label(search_, "Filter time zones");
    search_.signal_search_changed().connect(sigc::mem_fun(*this, &ZonePicker::on_query_changed));
    append(search_);

    list_.set_selection_mode(Gtk::SelectionMode::NONE);
    list_.set_activate_on_single_click(true);
    accessible_label(list_, "Time zones");
    empty_.add_css_class("zone-empty");
    empty_.set_xalign(0);
    empty_.set_wrap(true);
    list_.set_placeholder(empty_);

    auto add_row = [&](const std::string& city, const std::string& region, const std::string& offset) {
        auto* row = Gtk::make_managed<Gtk::ListBoxRow>();
        auto* box = Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 12);
        box->add_css_class("zone-option");
        auto* names = Gtk::make_managed<Gtk::Box>(Gtk::Orientation::VERTICAL, 1);
        names->set_hexpand(true);
        auto& city_label = label(city, "zone-city");
        city_label.set_ellipsize(Pango::EllipsizeMode::END);
        names->append(city_label);
        if (!region.empty()) {
            auto& region_label = label(region, "zone-region");
            region_label.set_ellipsize(Pango::EllipsizeMode::END);
            names->append(region_label);
        }
        auto& offset_label = label(offset, "zone-offset");
        offset_label.add_css_class("mono");
        offset_label.set_valign(Gtk::Align::CENTER);
        box->append(*names);
        box->append(offset_label);
        row->set_child(*box);
        list_.append(*row);
    };

    // The first row follows the device; it is listed only while not filtering.
    auto system = entry(zones::system_time_zone());
    add_row("Follow this device", system.city, system.offset_text);
    needles_.emplace_back();
    for (const auto& zone : all_zones()) {
        add_row(zone.city, zone.region, zone.offset_text);
        needles_.push_back(zone.haystack);
    }

    list_.set_filter_func(sigc::mem_fun(*this, &ZonePicker::filter));
    list_.signal_row_activated().connect([this](Gtk::ListBoxRow* row) {
        auto index = row->get_index();
        if (index == 0) {
            on_select_(std::nullopt);
        } else if (index > 0 && static_cast<std::size_t>(index) <= all_zones().size()) {
            on_select_(all_zones()[static_cast<std::size_t>(index) - 1].id);
        }
    });

    scroller_.add_css_class("zone-list");
    scroller_.set_policy(Gtk::PolicyType::NEVER, Gtk::PolicyType::AUTOMATIC);
    scroller_.set_min_content_height(260);
    scroller_.set_max_content_height(260);
    scroller_.set_child(list_);
    append(scroller_);
}

bool ZonePicker::filter(Gtk::ListBoxRow* row) {
    auto query = lower(std::string{search_.get_text()});
    std::ranges::replace(query, '_', ' ');
    auto needles = words(query);
    auto index = static_cast<std::size_t>(row->get_index());
    if (needles.empty()) return true;
    if (index == 0 || index >= needles_.size()) return false;
    return std::ranges::all_of(needles, [&](const std::string& word) {
        return needles_[index].find(word) != std::string::npos;
    });
}

void ZonePicker::on_query_changed() {
    auto query = std::string{search_.get_text()};
    auto first = query.find_first_not_of(" \t");
    auto last = query.find_last_not_of(" \t");
    auto trimmed = first == std::string::npos ? std::string{} : query.substr(first, last - first + 1);
    empty_.set_text("No time zone matches \"" + trimmed + "\".");
    list_.invalidate_filter();
}

void ZonePicker::set_value(const std::optional<std::string>& zone) {
    value_ = zone;
    std::size_t current = 0;
    if (zone) {
        const auto& zones = all_zones();
        auto it = std::ranges::find(zones, *zone, &ZoneEntry::id);
        current = it == zones.end() ? std::size_t(-1) : static_cast<std::size_t>(it - zones.begin()) + 1;
    }
    for (int i = 0; auto* row = list_.get_row_at_index(i); ++i) {
        if (static_cast<std::size_t>(i) == current) {
            row->add_css_class("current");
            gtk_accessible_update_state(GTK_ACCESSIBLE(row->gobj()), GTK_ACCESSIBLE_STATE_SELECTED, TRUE, -1);
        } else {
            row->remove_css_class("current");
            gtk_accessible_update_state(GTK_ACCESSIBLE(row->gobj()), GTK_ACCESSIBLE_STATE_SELECTED, FALSE, -1);
        }
    }
}

// ----------------------------------------------------------------- panel

SettingsPanel::SettingsPanel(Callbacks callbacks, bool can_keep_above)
    : callbacks_(std::move(callbacks)),
      zones_([this](std::optional<std::string> zone) { change([&](Settings& s) { s.time_zone = zone; }); }) {
    install_swatch_styles();
    set_visible(false);

    // The scrim covers the window; a click on it closes the panel.
    scrim_.add_css_class("settings-scrim");
    scrim_.set_hexpand(true);
    scrim_.set_vexpand(true);
    auto click = Gtk::GestureClick::create();
    click->signal_released().connect([this](int, double, double) { close(); });
    scrim_.add_controller(click);
    scrim_revealer_.set_transition_type(Gtk::RevealerTransitionType::CROSSFADE);
    scrim_revealer_.set_transition_duration(200);
    scrim_revealer_.set_child(scrim_);
    set_child(scrim_revealer_);

    sheet_revealer_.set_transition_type(Gtk::RevealerTransitionType::SLIDE_LEFT);
    sheet_revealer_.set_transition_duration(260);
    sheet_revealer_.set_halign(Gtk::Align::END);
    sheet_revealer_.set_vexpand(true);
    sheet_revealer_.set_child(sheet_);
    sheet_revealer_.property_child_revealed().signal_changed().connect([this] {
        if (!open_ && !sheet_revealer_.get_child_revealed()) set_visible(false);
    });
    add_overlay(sheet_revealer_);

    sheet_.add_css_class("sheet");
    sheet_.set_size_request(420, -1);
    sheet_.set_vexpand(true);
    accessible_label(sheet_, "Settings");

    // Header
    auto* header = Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL);
    header->add_css_class("sheet-header");
    auto& title = label("Settings", "sheet-title");
    title.set_hexpand(true);
    header->append(title);
    close_button_.add_css_class("close");
    close_button_.set_child(*Gtk::make_managed<Icon>(IconName::close));
    close_button_.set_tooltip_text("Close settings (Esc)");
    accessible_label(close_button_, "Close settings");
    close_button_.signal_clicked().connect([this] { close(); });
    header->append(close_button_);
    sheet_.append(*header);

    body_.add_css_class("sheet-body");
    body_scroller_.set_policy(Gtk::PolicyType::NEVER, Gtk::PolicyType::AUTOMATIC);
    body_scroller_.set_vexpand(true);
    body_scroller_.set_child(body_);
    sheet_.append(body_scroller_);

    // Release time zone
    {
        auto& box = section("Release time zone");
        box.append(note("The game unlocks at midnight local time, so the moment depends on where you are. Zones are "
                        "listed in the order they reach it."));
        box.append(zones_);
        zones_.search().signal_stop_search().connect([this] { close(); });
    }

    // Display
    {
        auto& box = section("Display");

        auto& theme_field = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::VERTICAL, 8);
        theme_field.append(label("Theme", "field-title"));
        auto& themes_row = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 8);
        themes_row.set_homogeneous(true);
        for (std::size_t i = 0; i < themes::all.size(); ++i) {
            auto theme = themes::all[i];
            auto& button = theme_buttons_[i];
            button.add_css_class("theme");
            if (i > 0) button.set_group(theme_buttons_[0]);
            auto& content = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::VERTICAL, 7);
            auto& swatch = *Gtk::make_managed<Gtk::Box>();
            swatch.add_css_class("swatch");
            swatch.add_css_class(std::format("swatch-{}", settings::key(theme)));
            content.append(swatch);
            auto& name = label(std::string{themes::info(theme).label});
            name.set_xalign(0.5);
            content.append(name);
            button.set_child(content);
            button.signal_toggled().connect([this, theme, &button] {
                if (!updating_ && button.get_active()) change([theme](Settings& s) { s.theme = theme; });
            });
            themes_row.append(button);
        }
        theme_field.append(themes_row);
        box.append(theme_field);

        auto& effects_field = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::VERTICAL, 8);
        effects_field.append(label("Animated backdrop", "field-title"));
        auto& segmented = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 6);
        segmented.add_css_class("segmented");
        segmented.set_homogeneous(true);
        constexpr std::array<const char*, 3> names{"Auto", "Low", "Off"};
        for (std::size_t i = 0; i < effect_levels.size(); ++i) {
            auto level = effect_levels[i];
            auto& button = effect_buttons_[i];
            button.set_label(names[i]);
            if (i > 0) button.set_group(effect_buttons_[0]);
            button.signal_toggled().connect([this, level, &button] {
                if (!updating_ && button.get_active()) change([level](Settings& s) { s.effects = level; });
            });
            segmented.append(button);
        }
        effects_field.append(segmented);
        effects_field.append(note("Auto lowers the rendering resolution when the frame rate drops. Off replaces the "
                                  "scene with a still gradient."));
        box.append(effects_field);

        box.append(switch_row("Milliseconds", "Adds a per-frame readout after the seconds.", millis_));
        millis_.property_active().signal_changed().connect([this] {
            if (!updating_) change([this](Settings& s) { s.show_milliseconds = millis_.get_active(); });
        });
    }

    // Alerts
    {
        auto& box = section("Alerts");
        box.append(switch_row("Notify me at release",
                              "Shows a system notification when the countdown reaches zero while the app is open.",
                              notify_));
        notify_.property_active().signal_changed().connect([this] {
            if (!updating_ && callbacks_.notify) callbacks_.notify(notify_.get_active());
        });
    }

    // Desktop
    {
        auto& box = section("Desktop");
        box.append(switch_row("Always on top",
                              can_keep_above ? "Keeps the window above other windows."
                                             : "Not available here: Wayland does not let apps keep a window above "
                                               "others.",
                              on_top_));
        on_top_.set_sensitive(can_keep_above);
        on_top_.property_active().signal_changed().connect([this] {
            if (!updating_) change([this](Settings& s) { s.always_on_top = on_top_.get_active(); });
        });
        box.append(switch_row("Network time", "Corrects the system clock against an NTP server at start-up.",
                              network_time_));
        network_time_.property_active().signal_changed().connect([this] {
            if (!updating_) change([this](Settings& s) { s.network_time = network_time_.get_active(); });
        });

        auto& sync_row = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 12);
        sync_text_.add_css_class("note");
        sync_text_.set_xalign(0);
        sync_text_.set_wrap(true);
        sync_text_.set_wrap_mode(Pango::WrapMode::WORD_CHAR);
        sync_text_.set_hexpand(true);
        accessible_label(sync_text_, "Clock status");
        sync_row.append(sync_text_);
        auto& sync_content = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 6);
        sync_content.append(*Gtk::make_managed<Icon>(IconName::sync, 15));
        sync_content.append(*Gtk::make_managed<Gtk::Label>("Sync now"));
        sync_button_.set_child(sync_content);
        sync_button_.add_css_class("flat-button");
        sync_button_.set_valign(Gtk::Align::CENTER);
        sync_button_.signal_clicked().connect([this] {
            if (callbacks_.sync) callbacks_.sync();
        });
        sync_row.append(sync_button_);
        box.append(sync_row);
    }

    // Keyboard shortcuts
    {
        auto& box = section("Keyboard shortcuts");
        auto& grid = *Gtk::make_managed<Gtk::Grid>();
        grid.set_row_spacing(8);
        grid.set_column_spacing(12);
        constexpr std::array<std::pair<const char*, const char*>, 6> shortcuts{{
            {"S", "Open or close settings"},
            {"F", "Toggle fullscreen"},
            {"M", "Toggle milliseconds"},
            {"T", "Next theme"},
            {"C", "Copy the countdown"},
            {"Esc", "Close settings"},
        }};
        int row = 0;
        for (const auto& [key, action] : shortcuts) {
            auto& kbd = *Gtk::make_managed<Gtk::Label>(key);
            kbd.add_css_class("kbd");
            kbd.add_css_class("mono");
            kbd.set_halign(Gtk::Align::START);
            grid.attach(kbd, 0, row);
            auto& text = label(action, "shortcut-action");
            grid.attach(text, 1, row);
            ++row;
        }
        box.append(grid);
    }

    // Reset
    {
        auto& box = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::VERTICAL);
        box.add_css_class("section");
        auto& reset = *Gtk::make_managed<Gtk::Button>("Restore defaults");
        reset.add_css_class("flat-button");
        reset.set_halign(Gtk::Align::START);
        reset.signal_clicked().connect([this] {
            if (callbacks_.change) callbacks_.change(Settings{});
        });
        box.append(reset);
        body_.append(box);
    }

    set_settings(settings_);
}

Gtk::Box& SettingsPanel::section(const char* title) {
    auto& box = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::VERTICAL, 12);
    box.add_css_class("section");
    box.append(label(upper(title), "label-caps"));
    body_.append(box);
    return box;
}

Gtk::Box& SettingsPanel::switch_row(const char* title, const char* hint, Gtk::Switch& control) {
    auto& row = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 16);
    auto& text = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::VERTICAL, 2);
    text.set_hexpand(true);
    auto& title_label = label(title, "switch-title");
    text.append(title_label);
    auto& hint_label = label(hint, "switch-hint");
    hint_label.set_wrap(true);
    hint_label.set_wrap_mode(Pango::WrapMode::WORD_CHAR);
    text.append(hint_label);
    row.append(text);
    control.set_valign(Gtk::Align::CENTER);
    accessible_label(control, title);
    row.append(control);
    return row;
}

void SettingsPanel::change(const std::function<void(Settings&)>& edit) {
    auto next = settings_;
    edit(next);
    if (callbacks_.change) callbacks_.change(next);
}

void SettingsPanel::set_settings(const Settings& settings) {
    settings_ = settings;
    updating_ = true;
    zones_.set_value(settings.time_zone);
    for (std::size_t i = 0; i < themes::all.size(); ++i) theme_buttons_[i].set_active(themes::all[i] == settings.theme);
    for (std::size_t i = 0; i < effect_levels.size(); ++i) {
        effect_buttons_[i].set_active(effect_levels[i] == settings.effects);
    }
    millis_.set_active(settings.show_milliseconds);
    notify_.set_active(settings.notify);
    on_top_.set_active(settings.always_on_top);
    network_time_.set_active(settings.network_time);
    updating_ = false;
}

void SettingsPanel::set_sync(const std::string& text, bool busy) {
    sync_text_.set_text(text);
    sync_button_.set_sensitive(settings_.network_time && !busy);
}

void SettingsPanel::set_sheet_width(int width) { sheet_.set_size_request(std::max(280, width), -1); }

void SettingsPanel::open() {
    if (open_) return;
    open_ = true;
    set_visible(true);
    scrim_revealer_.set_reveal_child(true);
    sheet_revealer_.set_reveal_child(true);
    close_button_.grab_focus();
}

void SettingsPanel::close() {
    if (!open_) return;
    open_ = false;
    scrim_revealer_.set_reveal_child(false);
    sheet_revealer_.set_reveal_child(false);
    zones_.search().set_text("");
    if (callbacks_.closed) callbacks_.closed();
}

}  // namespace gtavi

#pragma once
// Small helpers for the custom-drawn widgets: fonts, text layouts and shapes
// on a GtkSnapshot.

#include <gdkmm/rgba.h>
#include <gtk/gtk.h>
#include <pangomm/layout.h>

#include <algorithm>
#include <string>

namespace gtavi::drawing {

// The bundled families register under these names; the generic families
// cover a build that runs without the bundled fonts.
inline constexpr const char* sans_family = "Archivo SemiBold,Archivo,sans-serif";
inline constexpr const char* mono_family = "JetBrains Mono,monospace";

// A font at an absolute pixel size. `width` is the wdth axis in percent.
inline Pango::FontDescription font(const char* family, double px, int weight, int width = 100) {
    Pango::FontDescription description;
    description.set_family(family);
    description.set_absolute_size(px * PANGO_SCALE);
    description.set_weight(static_cast<Pango::Weight>(weight));
    // Variable fonts take the weight from the description; the width axis has
    // no Pango property that maps to a percentage, so it is set directly.
    description.set_variations("wght=" + std::to_string(weight) + ",wdth=" + std::to_string(width));
    return description;
}

struct LayoutOptions {
    double letter_spacing_px = 0;
    const char* features = nullptr;  // OpenType features, e.g. "tnum 1, lnum 1".
};

inline Glib::RefPtr<Pango::Layout> layout(Gtk::Widget& widget, const Glib::ustring& text,
                                          const Pango::FontDescription& description, LayoutOptions options = {}) {
    auto result = widget.create_pango_layout(text);
    result->set_font_description(description);
    if (options.letter_spacing_px != 0 || options.features != nullptr) {
        auto* list = pango_attr_list_new();
        if (options.letter_spacing_px != 0) {
            pango_attr_list_insert(list,
                                   pango_attr_letter_spacing_new(static_cast<int>(options.letter_spacing_px * PANGO_SCALE)));
        }
        if (options.features != nullptr) pango_attr_list_insert(list, pango_attr_font_features_new(options.features));
        pango_layout_set_attributes(result->gobj(), list);
        pango_attr_list_unref(list);
    }
    return result;
}

// Logical size of a layout in pixels.
struct Size {
    double width;
    double height;
};

inline Size size_of(const Glib::RefPtr<Pango::Layout>& layout) {
    int width = 0;
    int height = 0;
    layout->get_size(width, height);
    return {static_cast<double>(width) / PANGO_SCALE, static_cast<double>(height) / PANGO_SCALE};
}

inline void text(GtkSnapshot* s, const Glib::RefPtr<Pango::Layout>& layout, double x, double y, const Gdk::RGBA& color) {
    gtk_snapshot_save(s);
    graphene_point_t at{static_cast<float>(x), static_cast<float>(y)};
    gtk_snapshot_translate(s, &at);
    gtk_snapshot_append_layout(s, layout->gobj(), color.gobj());
    gtk_snapshot_restore(s);
}

inline graphene_rect_t rect(double x, double y, double width, double height) {
    return graphene_rect_t{{static_cast<float>(x), static_cast<float>(y)},
                           {static_cast<float>(width), static_cast<float>(height)}};
}

// Clips to a rounded rectangle; pop with gtk_snapshot_pop.
inline void push_rounded(GtkSnapshot* s, const graphene_rect_t& bounds, double radius) {
    GskRoundedRect rounded;
    float r = static_cast<float>(std::min<double>(radius, std::min(bounds.size.width, bounds.size.height) / 2));
    gsk_rounded_rect_init_from_rect(&rounded, &bounds, r);
    gtk_snapshot_push_rounded_clip(s, &rounded);
}

inline void rounded_rect(GtkSnapshot* s, const graphene_rect_t& bounds, double radius, const Gdk::RGBA& color) {
    push_rounded(s, bounds, radius);
    gtk_snapshot_append_color(s, color.gobj(), &bounds);
    gtk_snapshot_pop(s);
}

inline void circle(GtkSnapshot* s, double cx, double cy, double r, const Gdk::RGBA& color) {
    rounded_rect(s, rect(cx - r, cy - r, 2 * r, 2 * r), r, color);
}

inline GdkRGBA rgba(const Gdk::RGBA& color, double alpha) {
    return GdkRGBA{static_cast<float>(color.get_red()), static_cast<float>(color.get_green()),
                   static_cast<float>(color.get_blue()), static_cast<float>(alpha)};
}

}  // namespace gtavi::drawing

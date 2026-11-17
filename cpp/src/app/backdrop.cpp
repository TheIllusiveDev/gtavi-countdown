#include "backdrop.hpp"

#include <epoxy/gl.h>
#include <giomm/resource.h>
#include <glibmm/main.h>
#include <gdkmm/frameclock.h>
#include <gtk/gtk.h>
#include <gtkmm/snapshot.h>
#include <gtkmm/binlayout.h>
#include <gtkmm/glarea.h>
#include <gtkmm/settings.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <iostream>
#include <string>

#include "drawing.hpp"
#include "ui_colors.hpp"

namespace gtavi {

namespace {

// Device pixels are capped: the scene is soft and gains nothing beyond this.
constexpr double max_pixel_ratio = 1.5;
// Resolution scales the adaptive governor steps through, best first.
constexpr std::array<double, 5> scale_steps{1, 0.75, 0.55, 0.4, 0.3};
// A frame slower than this counts against the current resolution.
constexpr double slow_frame_ms = 1000.0 / 42;
// Consecutive slow frames tolerated before stepping the resolution down.
constexpr int slow_frame_limit = 24;
// The scene fades in over this long once it has drawn its first frame.
constexpr double fade_in_ms = 600;

std::string resource_text(const char* path) {
    auto bytes = Gio::Resource::lookup_data_global(path);
    gsize size = 0;
    auto* data = static_cast<const char*>(bytes->get_data(size));
    return {data, size};
}

bool animations_enabled() {
    auto settings = Gtk::Settings::get_default();
    return !settings || settings->property_gtk_enable_animations().get_value();
}

}  // namespace

// ------------------------------------------------------------------ scene

// Renders the shader into a GL area. Like the WebGL renderer it is ported
// from, it measures frame times and lowers its internal resolution when the
// GPU cannot keep up, and draws a single still frame when animations are off.
class Backdrop::Scene : public Gtk::GLArea {
public:
    Scene(const Palette& palette, Composition composition, EffectLevel effects)
        : palette_(palette), composition_(composition) {
        set_has_depth_buffer(false);
        set_has_stencil_buffer(false);
        set_can_target(false);
        set_effects(effects);
        animated_ = animations_enabled();
        // Without animations the scene appears at once instead of fading in.
        set_opacity(animated_ ? 0 : 1);
        if (auto settings = Gtk::Settings::get_default()) {
            animations_changed_ = settings->property_gtk_enable_animations().signal_changed().connect([this] {
                animated_ = animations_enabled();
                if (animated_) set_opacity(1);
                update_tick();
                queue_render();
            });
        }
    }

    ~Scene() override {
        animations_changed_.disconnect();
        failure_.disconnect();
        remove_tick();
    }

    // Called when OpenGL turns out to be unusable, so the gradient shows instead.
    sigc::signal<void()>& signal_failed() { return failed_; }

    void set_palette(const Palette& palette) {
        palette_ = palette;
        queue_render();
    }

    void set_composition(Composition composition) {
        composition_ = composition;
        queue_render();
    }

    void set_effects(EffectLevel effects) {
        if (effects == effects_) return;
        effects_ = effects;
        scale_index_ = effects == EffectLevel::low ? 2 : 0;
        slow_frames_ = 0;
        queue_render();
    }

protected:
    void on_realize() override {
        Gtk::GLArea::on_realize();
        make_current();
        if (has_error() || !build()) {
            fail();
            return;
        }
        epoch_ = g_get_monotonic_time();
        update_tick();
    }

    void on_unrealize() override {
        remove_tick();
        make_current();
        if (!has_error()) release();
        Gtk::GLArea::on_unrealize();
    }

    bool on_render(const Glib::RefPtr<Gdk::GLContext>&) override {
        if (program_ == 0) return true;
        draw();
        return true;
    }

private:
    bool on_tick(const Glib::RefPtr<Gdk::FrameClock>& clock) {
        auto now = clock->get_frame_time();
        // GTK does not draw a fully transparent widget, so the fade is timed
        // from the first tick rather than from the first drawn frame.
        if (get_opacity() < 1) {
            if (fade_start_ == 0) fade_start_ = now;
            set_opacity(std::min(1.0, static_cast<double>(now - fade_start_) / 1000.0 / fade_in_ms));
        }
        // The low preset halves the frame rate as well as the resolution.
        if (effects_ == EffectLevel::low && static_cast<double>(now - last_draw_) / 1000.0 < 1000.0 / 31) return true;
        if (effects_ == EffectLevel::automatic && last_frame_ != 0) {
            govern(static_cast<double>(now - last_frame_) / 1000.0);
        }
        last_frame_ = now;
        last_draw_ = now;
        queue_render();
        return true;
    }

    // Steps the resolution down after a sustained run of slow frames.
    void govern(double frame_ms) {
        // Long gaps are a minimised window or a stalled compositor, not rendering cost.
        if (frame_ms > 250) return;
        if (frame_ms > slow_frame_ms) {
            ++slow_frames_;
        } else {
            slow_frames_ = std::max(0, slow_frames_ - 2);
        }
        if (slow_frames_ >= slow_frame_limit && scale_index_ < scale_steps.size() - 1) {
            ++scale_index_;
            slow_frames_ = 0;
        }
    }

    GLuint compile(GLenum type, const std::string& source) {
        GLuint shader = glCreateShader(type);
        const char* text = source.c_str();
        glShaderSource(shader, 1, &text, nullptr);
        glCompileShader(shader);
        GLint ok = GL_FALSE;
        glGetShaderiv(shader, GL_COMPILE_STATUS, &ok);
        if (ok != GL_TRUE) {
            std::array<char, 2048> log{};
            glGetShaderInfoLog(shader, static_cast<GLsizei>(log.size()), nullptr, log.data());
            std::cerr << "Backdrop shader failed to compile: " << log.data() << '\n';
            glDeleteShader(shader);
            return 0;
        }
        return shader;
    }

    bool build() {
        // GLSL 1.50 is the language of OpenGL 3.2 core, the least GTK asks
        // for; GLSL ES 3.00 covers GLES contexts.
        bool es = get_context()->get_use_es();
        std::string header = es ? "#version 300 es\nprecision highp float;\n" : "#version 150\n";
        GLuint vertex = compile(GL_VERTEX_SHADER, header + resource_text("/app/gtavi/countdown/scene.vert"));
        GLuint fragment = compile(GL_FRAGMENT_SHADER, header + resource_text("/app/gtavi/countdown/scene.frag"));
        if (vertex == 0 || fragment == 0) {
            if (vertex != 0) glDeleteShader(vertex);
            if (fragment != 0) glDeleteShader(fragment);
            return false;
        }
        program_ = glCreateProgram();
        glAttachShader(program_, vertex);
        glAttachShader(program_, fragment);
        glBindAttribLocation(program_, 0, "aPosition");
        glLinkProgram(program_);
        // Shaders are owned by the program from here on.
        glDeleteShader(vertex);
        glDeleteShader(fragment);
        GLint ok = GL_FALSE;
        glGetProgramiv(program_, GL_LINK_STATUS, &ok);
        if (ok != GL_TRUE) {
            std::array<char, 2048> log{};
            glGetProgramInfoLog(program_, static_cast<GLsizei>(log.size()), nullptr, log.data());
            std::cerr << "Backdrop program failed to link: " << log.data() << '\n';
            release();
            return false;
        }

        // One oversized triangle covers the viewport with no diagonal seam.
        static constexpr std::array<GLfloat, 6> triangle{-1, -1, 3, -1, -1, 3};
        glGenVertexArrays(1, &vao_);
        glBindVertexArray(vao_);
        glGenBuffers(1, &vbo_);
        glBindBuffer(GL_ARRAY_BUFFER, vbo_);
        glBufferData(GL_ARRAY_BUFFER, sizeof(triangle), triangle.data(), GL_STATIC_DRAW);
        glEnableVertexAttribArray(0);
        glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 0, nullptr);
        glBindVertexArray(0);

        for (auto& [name, location] : locations_) location = glGetUniformLocation(program_, name);
        return true;
    }

    void release() {
        if (fbo_ != 0) glDeleteFramebuffers(1, &fbo_);
        if (texture_ != 0) glDeleteTextures(1, &texture_);
        if (vbo_ != 0) glDeleteBuffers(1, &vbo_);
        if (vao_ != 0) glDeleteVertexArrays(1, &vao_);
        if (program_ != 0) glDeleteProgram(program_);
        fbo_ = texture_ = vbo_ = vao_ = program_ = 0;
        target_width_ = target_height_ = 0;
    }

    void fail() {
        release();
        // The owner removes this widget, which must not happen inside realize.
        failure_ = Glib::signal_idle().connect([this] {
            failed_.emit();
            return false;
        });
    }

    // Animated scenes redraw on every frame; still scenes only when something changes.
    void update_tick() {
        if (animated_ && program_ != 0 && tick_ == 0) {
            last_frame_ = 0;
            tick_ = add_tick_callback([this](const Glib::RefPtr<Gdk::FrameClock>& clock) { return on_tick(clock); });
        } else if (!animated_) {
            remove_tick();
        }
    }

    void remove_tick() {
        if (tick_ != 0) remove_tick_callback(tick_);
        tick_ = 0;
    }

    // Makes sure the offscreen target exists at the given size.
    void ensure_target(int width, int height) {
        if (fbo_ != 0 && width == target_width_ && height == target_height_) return;
        if (fbo_ == 0) glGenFramebuffers(1, &fbo_);
        if (texture_ == 0) glGenTextures(1, &texture_);
        glBindTexture(GL_TEXTURE_2D, texture_);
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, width, height, 0, GL_RGBA, GL_UNSIGNED_BYTE, nullptr);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glBindFramebuffer(GL_FRAMEBUFFER, fbo_);
        glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, texture_, 0);
        target_width_ = width;
        target_height_ = height;
    }

    void draw() {
        int scale = get_scale_factor();
        int output_width = get_width() * scale;
        int output_height = get_height() * scale;
        if (output_width <= 0 || output_height <= 0) return;

        double ratio = std::min<double>(scale, max_pixel_ratio) * scale_steps[scale_index_];
        int width = std::max(1, static_cast<int>(std::lround(get_width() * ratio)));
        int height = std::max(1, static_cast<int>(std::lround(get_height() * ratio)));
        bool offscreen = width != output_width || height != output_height;

        GLint output = 0;
        glGetIntegerv(GL_DRAW_FRAMEBUFFER_BINDING, &output);
        if (offscreen) {
            ensure_target(width, height);
            glBindFramebuffer(GL_FRAMEBUFFER, fbo_);
        }
        glViewport(0, 0, width, height);
        glDisable(GL_DEPTH_TEST);
        glDisable(GL_BLEND);
        glUseProgram(program_);

        // Still frames use a fixed time so that they are reproducible.
        float seconds = animated_ ? static_cast<float>(g_get_monotonic_time() - epoch_) / 1e6f : 12.0f;
        auto vec3 = [&](const char* name, Rgb c) { glUniform3f(location(name), c.r, c.g, c.b); };
        glUniform2f(location("uResolution"), static_cast<float>(width), static_cast<float>(height));
        glUniform1f(location("uTime"), seconds);
        vec3("uSkyTop", palette_.sky_top);
        vec3("uSkyMid", palette_.sky_mid);
        vec3("uSkyLow", palette_.sky_low);
        vec3("uSunHot", palette_.sun_hot);
        vec3("uSunCool", palette_.sun_cool);
        vec3("uGlow", palette_.glow);
        vec3("uWater", palette_.water);
        glUniform1f(location("uStars"), palette_.stars);
        glUniform1f(location("uBands"), palette_.bands);
        glUniform1f(location("uHorizon"), 0.5f - composition_.horizon);
        glUniform1f(location("uSunRadius"), composition_.sun_radius);

        glBindVertexArray(vao_);
        glDrawArrays(GL_TRIANGLES, 0, 3);
        glBindVertexArray(0);

        if (offscreen) {
            glBindFramebuffer(GL_READ_FRAMEBUFFER, fbo_);
            glBindFramebuffer(GL_DRAW_FRAMEBUFFER, static_cast<GLuint>(output));
            glBlitFramebuffer(0, 0, width, height, 0, 0, output_width, output_height, GL_COLOR_BUFFER_BIT, GL_LINEAR);
            glBindFramebuffer(GL_FRAMEBUFFER, static_cast<GLuint>(output));
        }
    }

    GLint location(const char* name) const {
        for (const auto& [key, value] : locations_) {
            if (std::string_view{key} == name) return value;
        }
        return -1;
    }

    Palette palette_;
    Composition composition_;
    EffectLevel effects_ = EffectLevel::off;  // Replaced in the constructor.
    bool animated_ = true;
    std::size_t scale_index_ = 0;
    int slow_frames_ = 0;
    gint64 epoch_ = 0;
    gint64 last_frame_ = 0;
    gint64 last_draw_ = 0;
    gint64 fade_start_ = 0;
    guint tick_ = 0;

    GLuint program_ = 0;
    GLuint vao_ = 0;
    GLuint vbo_ = 0;
    GLuint fbo_ = 0;
    GLuint texture_ = 0;
    int target_width_ = 0;
    int target_height_ = 0;
    std::array<std::pair<const char*, GLint>, 13> locations_{{
        {"uResolution", -1}, {"uTime", -1}, {"uSkyTop", -1}, {"uSkyMid", -1}, {"uSkyLow", -1},
        {"uSunHot", -1},     {"uSunCool", -1}, {"uGlow", -1}, {"uWater", -1}, {"uStars", -1},
        {"uBands", -1},      {"uHorizon", -1}, {"uSunRadius", -1},
    }};

    sigc::signal<void()> failed_;
    sigc::connection failure_;
    sigc::connection animations_changed_;
};

// --------------------------------------------------------------- backdrop

Backdrop::Backdrop(Theme theme, EffectLevel effects)
    : Glib::ObjectBase("GtaviBackdrop"), theme_(theme), effects_(effects) {
    set_layout_manager(Gtk::BinLayout::create());
    set_can_target(false);
    set_hexpand(true);
    set_vexpand(true);
    gtk_accessible_update_state(GTK_ACCESSIBLE(gobj()), GTK_ACCESSIBLE_STATE_HIDDEN, TRUE, -1);
    if (effects_ != EffectLevel::off) mount();
}

Backdrop::~Backdrop() { unmount(); }

// The renderer is created when effects are enabled and torn down when they
// are switched off, so that no GPU memory or context is held needlessly. A
// theme change does not rebuild the context; it only swaps the palette.
void Backdrop::mount() {
    if (scene_) return;
    scene_ = std::make_unique<Scene>(themes::info(theme_).palette, composition_, effects_);
    scene_->signal_failed().connect([this] { unmount(); });
    scene_->set_parent(*this);
}

void Backdrop::unmount() {
    if (!scene_) return;
    scene_->unparent();
    scene_.reset();
    queue_draw();
}

void Backdrop::set_theme(Theme theme) {
    if (theme == theme_) return;
    theme_ = theme;
    if (scene_) scene_->set_palette(themes::info(theme).palette);
    queue_draw();
}

void Backdrop::set_effects(EffectLevel effects) {
    if (effects == effects_) return;
    effects_ = effects;
    if (effects == EffectLevel::off) {
        unmount();
    } else if (scene_) {
        scene_->set_effects(effects);
    } else {
        mount();
    }
}

void Backdrop::set_composition(Composition composition) {
    if (composition == composition_) return;
    composition_ = composition;
    if (scene_) scene_->set_composition(composition);
    queue_draw();
}

void Backdrop::snapshot_vfunc(const Glib::RefPtr<Gtk::Snapshot>& snapshot) {
    auto* s = snapshot->gobj();
    float width = static_cast<float>(get_width());
    float height = static_cast<float>(get_height());
    auto bounds = drawing::rect(0, 0, width, height);
    graphene_point_t top = {0, 0};
    graphene_point_t bottom = {0, height};

    // The still gradient, under the scene so that it shows while the scene
    // fades in and wherever OpenGL is unavailable. Stops placed relative to
    // the horizon follow the composition; out-of-order stops are clamped to
    // the previous one, as in CSS.
    const auto& fallback = themes::info(theme_).fallback;
    std::array<GskColorStop, fallback.size()> stops{};
    float previous = 0;
    for (std::size_t i = 0; i < fallback.size(); ++i) {
        const auto& stop = fallback[i];
        float offset = stop.at_horizon ? composition_.horizon + stop.position : stop.position;
        offset = std::clamp(offset, previous, 1.0f);
        previous = offset;
        stops[i] = {offset, GdkRGBA{stop.color.r, stop.color.g, stop.color.b, 1}};
    }
    gtk_snapshot_append_linear_gradient(s, &bounds, &top, &bottom, stops.data(), stops.size());

    if (scene_) snapshot_child(*scene_, snapshot);

    // The scrim darkens the top and bottom edges so that text over the scene
    // keeps its contrast regardless of theme.
    auto scrim = ui_colors(theme_).scrim;
    std::array<GskColorStop, 2> fade_down{{{0.0f, drawing::rgba(scrim, 0.5)}, {0.22f, drawing::rgba(scrim, 0)}}};
    gtk_snapshot_append_linear_gradient(s, &bounds, &top, &bottom, fade_down.data(), fade_down.size());
    std::array<GskColorStop, 2> fade_up{{{0.0f, drawing::rgba(scrim, 0.72)}, {0.46f, drawing::rgba(scrim, 0)}}};
    gtk_snapshot_append_linear_gradient(s, &bounds, &bottom, &top, fade_up.data(), fade_up.size());
}

}  // namespace gtavi

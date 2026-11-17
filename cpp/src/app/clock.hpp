#pragma once
// A wall clock that ticks once per second, in phase with a target.
//
// now() only changes when the displayed seconds value changes, so the UI
// redraws its text at most once per second; the milliseconds readout reads
// precise() from a frame callback instead. Every tick re-reads the system
// clock, so a suspended laptop or a late timer can delay an update but never
// skew it.

#include <glibmm/main.h>
#include <sigc++/signal.h>

#include <cstdint>

namespace gtavi {

class Clock {
public:
    Clock();
    ~Clock();
    Clock(const Clock&) = delete;
    Clock& operator=(const Clock&) = delete;

    // Corrected current time in Unix milliseconds, updated once per tick.
    std::int64_t now() const { return now_; }

    // Correction added to the system clock, in milliseconds, from a network
    // time measurement; zero when the system clock is trusted.
    double offset() const { return offset_; }

    // Corrected current time, read directly from the system clock.
    std::int64_t precise() const;

    void start(std::int64_t target);
    // Re-aligns the tick with a new target, for example after a zone change.
    void retarget(std::int64_t target);
    // Applies a new clock correction and re-aligns the tick immediately.
    void set_offset(double offset);
    // Reads the clock now and schedules the next tick on the next change of the display.
    void resync();
    void stop();

    // Emitted after every tick.
    sigc::signal<void()>& signal_ticked() { return ticked_; }

private:
    std::int64_t now_ = 0;
    double offset_ = 0;
    std::int64_t target_ = 0;
    bool running_ = false;
    sigc::connection timer_;
    sigc::signal<void()> ticked_;
};

}  // namespace gtavi

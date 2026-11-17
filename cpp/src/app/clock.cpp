#include "clock.hpp"

#include <chrono>
#include <cmath>

#include "countdown.hpp"

namespace gtavi {

Clock::Clock() { now_ = precise(); }

Clock::~Clock() { stop(); }

std::int64_t Clock::precise() const {
    using namespace std::chrono;
    auto system = duration_cast<milliseconds>(system_clock::now().time_since_epoch()).count();
    return system + std::llround(offset_);
}

void Clock::start(std::int64_t target) {
    target_ = target;
    running_ = true;
    resync();
}

void Clock::retarget(std::int64_t target) {
    target_ = target;
    resync();
}

void Clock::set_offset(double offset) {
    offset_ = offset;
    resync();
}

void Clock::resync() {
    if (!running_) return;
    now_ = precise();
    timer_.disconnect();
    // Timers may fire a fraction of a millisecond early. Should that happen,
    // this simply runs again after a millisecond or two.
    auto wait = countdown::ms_until_next_second(now_, target_);
    timer_ = Glib::signal_timeout().connect(
        [this] {
            resync();
            return false;
        },
        static_cast<unsigned>(wait), Glib::PRIORITY_HIGH);
    ticked_.emit();
}

void Clock::stop() {
    running_ = false;
    timer_.disconnect();
}

}  // namespace gtavi

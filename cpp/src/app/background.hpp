#pragma once
// Runs blocking work (network time, mostly) off the main thread and hands
// the result back to the main loop.

#include <exception>
#include <expected>
#include <functional>
#include <memory>
#include <string>
#include <thread>

namespace gtavi {

// Queues `callback` to run once on the GLib main loop. Safe to call from any thread.
void post_to_main(std::function<void()> callback);

// Runs `work` on a detached thread and calls `done` on the main loop with its
// result, or with the message of the exception it threw. `done` is dropped
// when `alive` has expired by then, so a closed window is never touched.
template <typename T, typename Work, typename Done>
void run_in_background(std::weak_ptr<void> alive, Work work, Done done) {
    std::thread([alive = std::move(alive), work = std::move(work), done = std::move(done)]() mutable {
        std::expected<T, std::string> result = [&]() -> std::expected<T, std::string> {
            try {
                return work();
            } catch (const std::exception& e) {
                return std::unexpected(std::string{e.what()});
            }
        }();
        post_to_main([alive = std::move(alive), done = std::move(done), result = std::move(result)]() mutable {
            if (!alive.expired()) done(std::move(result));
        });
    }).detach();
}

}  // namespace gtavi

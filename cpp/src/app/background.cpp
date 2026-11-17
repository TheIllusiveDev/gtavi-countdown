#include "background.hpp"

#include <glib.h>

namespace gtavi {

void post_to_main(std::function<void()> callback) {
    auto* boxed = new std::function<void()>(std::move(callback));
    g_idle_add_full(
        G_PRIORITY_DEFAULT,
        [](gpointer data) -> gboolean {
            (*static_cast<std::function<void()>*>(data))();
            return G_SOURCE_REMOVE;
        },
        boxed, [](gpointer data) { delete static_cast<std::function<void()>*>(data); });
}

}  // namespace gtavi

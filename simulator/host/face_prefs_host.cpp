#include "platform/face_prefs.h"

namespace {

clock_face::Options g_opts{};

}  // namespace

void face_prefs::reload() {}

clock_face::Options face_prefs::options() { return g_opts; }

namespace sim_face { void seconds(bool on) { g_opts.time = on ? clock_face::TimeFormat::Hms : clock_face::TimeFormat::Hm; } }

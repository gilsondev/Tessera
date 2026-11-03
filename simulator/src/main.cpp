#include "host/sim_hooks.h"
#include "platform/settings.h"
#include "scenario.h"
#include "sim_app.h"
#include "sim_fonts.h"
#include "ui/screensaver.h"

#include <lvgl.h>
#include <drivers/sdl/lv_sdl_mouse.h>
#include <drivers/sdl/lv_sdl_window.h>

#include <SDL.h>

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <string>
#include <fstream>
#include <sstream>

namespace {

struct Args {
    int width = 320;
    int height = 480;
    std::string scenario = "normal";
    std::string fixtures_dir;
    std::string screenshot;
    std::string settings_dir = "/tmp/tessera-sim-settings";
    std::string screen = "home";
    bool headless = false;
    bool clock_check = false;
    int frames = 8;
    std::string journey;
};

void usage(const char *argv0) {
    std::fprintf(stderr,
                 "usage: %s [--viewport WxH] [--scenario name] [--fixtures dir] "
                 "[--screen home|todoist|weather|aitokens|calendar|clock|media|focus|settings] [--journey path.txt] [--screenshot path.bmp] "
                 "[--settings dir] [--headless]\n",
                 argv0);
}

bool parse_args(int argc, char **argv, Args &a) {
    for (int i = 1; i < argc; ++i) {
        if (!std::strcmp(argv[i], "--viewport") && i + 1 < argc) {
            int w = 0, h = 0;
            if (std::sscanf(argv[++i], "%dx%d", &w, &h) != 2 || w < 160 || h < 160) return false;
            a.width = w;
            a.height = h;
        } else if (!std::strcmp(argv[i], "--scenario") && i + 1 < argc) {
            a.scenario = argv[++i];
        } else if (!std::strcmp(argv[i], "--fixtures") && i + 1 < argc) {
            a.fixtures_dir = argv[++i];
        } else if (!std::strcmp(argv[i], "--screenshot") && i + 1 < argc) {
            a.screenshot = argv[++i];
            a.headless = true;
        } else if (!std::strcmp(argv[i], "--settings") && i + 1 < argc) {
            a.settings_dir = argv[++i];
        } else if (!std::strcmp(argv[i], "--screen") && i + 1 < argc) {
            a.screen = argv[++i];
        } else if (!std::strcmp(argv[i], "--journey") && i + 1 < argc) {
            a.journey = argv[++i]; a.headless = true;
        } else if (!std::strcmp(argv[i], "--clock-check")) {
            a.clock_check = true; a.headless = true;
        } else if (!std::strcmp(argv[i], "--headless")) {
            a.headless = true;
        } else if (!std::strcmp(argv[i], "--help") || !std::strcmp(argv[i], "-h")) {
            usage(argv[0]);
            std::exit(0);
        } else {
            return false;
        }
    }
    return true;
}

std::string fixture_path(const Args &a) {
    std::string dir = a.fixtures_dir;
    if (dir.empty()) dir = "fixtures";
    return dir + "/" + a.scenario + ".json";
}

void apply_scenario(const Scenario &s) {
    sim_time::set(s.hour, s.minute, s.wday, s.day, s.month, s.time_ok);
    sim_power::set_battery(s.battery_percent);
}

bool save_bmp(const char *path, int w, int h) {
    lv_display_t *disp = lv_display_get_default();
    if (!disp) return false;
    lv_refr_now(disp);
    auto *ren = static_cast<SDL_Renderer *>(lv_sdl_window_get_renderer(disp));
    if (!ren) {
        std::fprintf(stderr, "sim: no SDL renderer for screenshot\n");
        return false;
    }
    SDL_Surface *tmp = SDL_CreateRGBSurfaceWithFormat(0, w, h, 32, SDL_PIXELFORMAT_ARGB8888);
    if (!tmp) return false;
    if (SDL_RenderReadPixels(ren, nullptr, SDL_PIXELFORMAT_ARGB8888, tmp->pixels, tmp->pitch) != 0) {
        std::fprintf(stderr, "sim: RenderReadPixels: %s\n", SDL_GetError());
        SDL_FreeSurface(tmp);
        return false;
    }
    int rc = SDL_SaveBMP(tmp, path);
    SDL_FreeSurface(tmp);
    return rc == 0;
}

void print_report(const Scenario &s, const sim_app::State &app) {
    std::printf(
        "scenario=%s screen=%s tiles=%d todoist=%d weather=%d aitokens=%d clock_tile=%d wifi=%d bat=%d\n",
        s.name.empty() ? "?" : s.name.c_str(), sim_app::screen_name(app.screen), app.tile_count, app.has_todoist,
        app.has_weather, app.has_aitokens, app.has_clock_tile, s.wifi_online ? 1 : 0, s.battery_percent);
}

struct Pointer { int x = 0, y = 0; bool pressed = false; } pointer;
void pointer_read(lv_indev_t *, lv_indev_data_t *data) {
    data->point.x = pointer.x; data->point.y = pointer.y;
    data->state = pointer.pressed ? LV_INDEV_STATE_PRESSED : LV_INDEV_STATE_RELEASED;
}
void frames(int n = 6) { for (int i=0;i<n;++i) { lv_timer_handler(); SDL_Delay(16); } }
bool has_text(lv_obj_t *obj, const std::string &text) {
    if (lv_obj_has_flag(obj,LV_OBJ_FLAG_HIDDEN)) return false;
    if (lv_obj_check_type(obj,&lv_label_class) && std::string(lv_label_get_text(obj)).find(text) != std::string::npos) return true;
    for (uint32_t i=0;i<lv_obj_get_child_count(obj);++i) if (has_text(lv_obj_get_child(obj,i),text)) return true;
    return false;
}
bool run_journey(const Args &args, sim_app::State &app) {
    auto *input = lv_indev_create(); lv_indev_set_type(input,LV_INDEV_TYPE_POINTER); lv_indev_set_read_cb(input,pointer_read);
    std::ifstream file(args.journey); if (!file) return false;
    std::string line; int step=0;
    while (std::getline(file,line)) {
        ++step; if (line.empty() || line[0]=='#') continue;
        std::istringstream in(line); std::string command; in>>command;
        if (command=="tap") { in>>pointer.x>>pointer.y; pointer.pressed=true; frames(4); pointer.pressed=false; frames(8); }
        else if (command=="drag") {
            int x1,y1,x2,y2; in>>x1>>y1>>x2>>y2; pointer={x1,y1,true}; frames(3);
            for(int i=1;i<=20;++i) { pointer.x=x1+(x2-x1)*i/20; pointer.y=y1+(y2-y1)*i/20; frames(1); }
            pointer.pressed=false; frames(20);
        } else if (command=="expect-screen") {
            std::string expected; in>>expected;
            if (expected != sim_app::screen_name(app.screen)) { std::fprintf(stderr,"journey step %d: expected screen %s, got %s\n",step,expected.c_str(),sim_app::screen_name(app.screen)); return false; }
        } else if (command=="expect-text") {
            std::string expected; std::getline(in>>std::ws,expected);
            if (!has_text(lv_screen_active(),expected)) { std::fprintf(stderr,"journey step %d: missing text %s\n",step,expected.c_str()); return false; }
        } else if (command=="media") {
            std::string wire; std::getline(in>>std::ws,wire);
            // Journey files use | as a readable tab separator.
            for(char &c: wire) if(c=='|') c='\t';
            sim_app::ingest_media(app,wire.c_str()); frames();
        } else if (command=="snapshot") {
            std::string name; in>>name;
            const char *out = std::getenv("SIM_OUT"); std::string path = std::string(out?out:"/tmp/tessera-sim-out") + "/" + name;
            if (!save_bmp(path.c_str(),args.width,args.height)) return false;
        } else if (command=="wait") { int n; in>>n; frames(n); }
        else { std::fprintf(stderr,"unknown journey command %s\n",command.c_str()); return false; }
        std::printf("journey step %d OK: %s\n",step,command.c_str());
    }
    lv_indev_delete(input); return true;
}

uint32_t clock_test_ms = 0;
bool clock_check(const Args &args) {
    lv_tick_set_cb([]() -> uint32_t { return clock_test_ms; });
    sim_face::seconds(true);
    sim_time::set(23, 59, 2, 3, 10);
    sim_time::seconds(58);
    screensaver::show();
    auto step = [](int ms) {
        for (int i=0; i<ms; i+=16) {
            clock_test_ms += 16;
            screensaver::tick(clock_test_ms);
            lv_timer_handler();
        }
    };
    step(700);
    sim_time::seconds(59); step(16);
    if (!screensaver::timing().active) return false;
    // Miss three seconds while a flap is mid-flight. The first resumed frame
    // must show current time with no old animation left to replay.
    clock_test_ms += 3000;
    sim_time::set(0, 0, 3, 4, 10); sim_time::seconds(2);
    screensaver::tick(clock_test_ms); lv_timer_handler();
    auto state = screensaver::timing();
    if (state.active || std::strcmp(state.digits,"000002") || state.snaps != 1) return false;
    step(400);
    sim_time::seconds(3); step(16); step(400);
    state = screensaver::timing();
    if (state.active || std::strcmp(state.digits,"000003")) return false;
    // NTP backward correction must also settle immediately, without replay.
    sim_time::seconds(1); step(16);
    state = screensaver::timing();
    if (state.active || std::strcmp(state.digits,"000001")) return false;
    if (!args.screenshot.empty() && !save_bmp(args.screenshot.c_str(), args.width, args.height)) return false;
    sim_face::seconds(false); step(16);
    sim_time::set(0,1,3,4,10); step(16); step(500);
    state = screensaver::timing();
    if (state.active || std::strcmp(state.digits,"0001")) return false;
    std::printf("clock-check PASS: normal flips, interrupted flip, three-second stall, midnight, NTP correction, HH:MM\n");
    return true;
}

}  // namespace

int main(int argc, char **argv) {
    Args args;
    if (!parse_args(argc, argv, args)) {
        usage(argv[0]);
        return 2;
    }

    if (args.headless) {
        setenv("SDL_VIDEODRIVER", "dummy", 0);
    }

    sim_settings::set_dir(args.settings_dir.c_str());

    std::string path = fixture_path(args);
    Scenario scenario;
    if (!load_scenario(path, scenario)) {
        std::fprintf(stderr, "sim: failed to load %s\n", path.c_str());
        return 1;
    }
    if (scenario.name.empty()) scenario.name = args.scenario;
    apply_scenario(scenario);
    settings::save_dashboard(scenario.dashboard);

    lv_init();
    sim_fonts::init();
    lv_display_t *disp = lv_sdl_window_create(args.width, args.height);
    if (!disp) {
        std::fprintf(stderr, "sim: lv_sdl_window_create failed\n");
        return 1;
    }
    lv_sdl_window_set_title(disp, ("Tessera sim — " + scenario.name).c_str());
    lv_indev_t *mouse = lv_sdl_mouse_create();
    (void)mouse;

    sim_app::State app;
    sim_app::init(app, scenario);
    if (args.clock_check) return clock_check(args) ? 0 : 6;

    const bool expect_todoist = dashboard::is_enabled(scenario.dashboard, dashboard::id_todoist());
    const bool expect_weather = dashboard::is_enabled(scenario.dashboard, dashboard::id_weather());
    const bool expect_ai = dashboard::is_enabled(scenario.dashboard, dashboard::id_aitokens());
    const bool expect_clock = dashboard::is_enabled(scenario.dashboard, dashboard::id_clock());
    if (app.has_todoist != expect_todoist || app.has_weather != expect_weather || app.has_aitokens != expect_ai ||
        app.has_clock_tile != expect_clock) {
        std::fprintf(stderr, "sim: compose mismatch vs dashboard fixture\n");
        return 3;
    }

    sim_app::Screen want = sim_app::parse_screen(args.screen.c_str());
    if (want != sim_app::Screen::Home) sim_app::show(app, want);
    print_report(scenario, app);

    for (int i = 0; i < args.frames; ++i) {
        lv_timer_handler();
        SDL_Delay(16);
    }

    if (!args.journey.empty() && !run_journey(args,app)) { sim_app::destroy(app); return 5; }

    if (!args.screenshot.empty()) {
        if (!save_bmp(args.screenshot.c_str(), args.width, args.height)) {
            std::fprintf(stderr, "sim: screenshot failed (%s)\n", args.screenshot.c_str());
            return 4;
        }
        std::printf("screenshot=%s\n", args.screenshot.c_str());
        sim_app::destroy(app);
        return 0;
    }

    if (args.headless) {
        sim_app::destroy(app);
        return 0;
    }

    while (!app.quit_requested) {
        lv_timer_handler();
        SDL_Event e;
        while (SDL_PollEvent(&e)) {
            if (e.type == SDL_QUIT) app.quit_requested = true;
            if (e.type == SDL_KEYDOWN && e.key.keysym.sym == SDLK_ESCAPE) sim_app::on_escape(app);
        }
        SDL_Delay(16);
    }
    sim_app::destroy(app);
    return 0;
}

//! Metrics collected by calling into the Lua VM on the main thread.
//!
//! - Player count: polled every 60 s via `#player.GetAll()`
//! - Lua memory:   polled every 300 s via `collectgarbage("count")`
//!
//! Values are stored in atomics so the Prometheus refresh thread can read them
//! lock-free.

use std::sync::atomic::{AtomicU64, Ordering};

use gmod::lua::State as LuaState;

static PLAYER_COUNT: AtomicU64 = AtomicU64::new(0);
static LUA_MEMORY_KIB: AtomicU64 = AtomicU64::new(0);

pub fn player_count() -> f64 {
    f64::from_bits(PLAYER_COUNT.load(Ordering::Relaxed))
}

pub fn lua_memory_kib() -> f64 {
    f64::from_bits(LUA_MEMORY_KIB.load(Ordering::Relaxed))
}

/// Call once from gmod13_open to install the collection timers.
pub unsafe fn install(lua: LuaState) {
    // Register the C callbacks
    lua.push_function(poll_players);
    let poll_players_ref = lua.reference();

    lua.push_function(poll_lua_memory);
    let poll_lua_memory_ref = lua.reference();

    // timer.Create("gmsv_serverstat_players", 60, 0, poll_players)
    lua.get_global(lua_string!("timer"));

    lua.get_field(-1, lua_string!("Create"));
    lua.push_string("gmsv_serverstat_players");
    lua.push_number(60.0);
    lua.push_number(0.0);
    lua.from_reference(poll_players_ref);
    lua.call(4, 0);

    // timer.Create("gmsv_serverstat_luamem", 300, 0, poll_lua_memory)
    lua.get_field(-1, lua_string!("Create"));
    lua.push_string("gmsv_serverstat_luamem");
    lua.push_number(300.0);
    lua.push_number(0.0);
    lua.from_reference(poll_lua_memory_ref);
    lua.call(4, 0);

    lua.pop(); // pop timer table

    lua.dereference(poll_players_ref);
    lua.dereference(poll_lua_memory_ref);

    // Initial read
    poll_players(lua);
    poll_lua_memory(lua);
}

/// Called from gmod13_close to remove timers.
pub unsafe fn uninstall(lua: LuaState) {
    lua.get_global(lua_string!("timer"));

    lua.get_field(-1, lua_string!("Remove"));
    lua.push_string("gmsv_serverstat_players");
    lua.call(1, 0);

    lua.get_field(-1, lua_string!("Remove"));
    lua.push_string("gmsv_serverstat_luamem");
    lua.call(1, 0);

    lua.pop();
}

/// `#player.GetAll()`
unsafe extern "C-unwind" fn poll_players(lua: LuaState) -> i32 {
    lua.get_global(lua_string!("player"));
    lua.get_field(-1, lua_string!("GetAll"));
    lua.call(0, 1); // stack: [player, result_table]
    let count = lua.len(-1);
    lua.pop_n(2); // pop result_table and player
    PLAYER_COUNT.store((count as f64).to_bits(), Ordering::Relaxed);
    0
}

/// `collectgarbage("count")` — returns KiB
unsafe extern "C-unwind" fn poll_lua_memory(lua: LuaState) -> i32 {
    lua.get_global(lua_string!("collectgarbage"));
    lua.push_string("count");
    lua.call(1, 1);
    let kib = lua.to_number(-1);
    lua.pop();
    LUA_MEMORY_KIB.store(kib.to_bits(), Ordering::Relaxed);
    0
}

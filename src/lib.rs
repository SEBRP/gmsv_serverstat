#![allow(unsafe_op_in_unsafe_fn)]

#[macro_use]
extern crate gmod;

mod lua_metrics;
mod prometheus;
mod sysinfo;

#[cfg(feature = "lua-api")]
mod lua_api;

#[cfg(not(feature = "lua-api"))]
mod entry {
    use gmod::lua::State as LuaState;

    #[gmod13_open]
    unsafe fn gmod13_open(lua: LuaState) -> i32 {
        crate::prometheus::start();
        crate::lua_metrics::install(lua);
        0
    }

    #[gmod13_close]
    unsafe fn gmod13_close(lua: LuaState) -> i32 {
        crate::prometheus::stop();
        crate::lua_metrics::uninstall(lua);
        0
    }
}

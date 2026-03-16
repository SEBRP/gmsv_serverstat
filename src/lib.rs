#![allow(unsafe_op_in_unsafe_fn)]

#[macro_use]
extern crate gmod;

mod sysinfo;
mod prometheus;

#[cfg(feature = "lua-api")]
mod lua_api;

#[cfg(not(feature = "lua-api"))]
mod entry {
	use gmod::lua::State as LuaState;

	#[gmod13_open]
	unsafe fn gmod13_open(_lua: LuaState) -> i32 {
		crate::prometheus::start();
		0
	}

	#[gmod13_close]
	unsafe fn gmod13_close(_lua: LuaState) -> i32 {
		crate::prometheus::stop();
		0
	}
}

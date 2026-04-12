pub(crate) mod r#async;
pub(crate) mod export;
pub(crate) mod realtime;

use gmod::lua::State as LuaState;

#[gmod13_open]
unsafe fn gmod13_open(lua: LuaState) -> i32 {
    crate::prometheus::start();

    lua.new_table(); // serverstat
    lua.new_table(); // async
    lua.new_table(); // realtime

    // Stack layout: [ serverstat(-3) | async(-2) | realtime(-1) ]
    // After pushing a value the indices shift by one:
    //   [ serverstat(-4) | async(-3) | realtime(-2) | value(-1) ]

    macro_rules! push_api_func {
        ($func:ident, $name:literal) => {
            lua.push_function(export::sync::$func);
            lua.set_field(-4, lua_string!($name)); // serverstat

            lua.push_function(export::r#async::$func);
            lua.set_field(-3, lua_string!($name)); // async
        };
    }

    macro_rules! push_realtime_api_func {
        ($func:ident, $name:literal) => {
            lua.push_function(export::sync::$func);
            lua.set_field(-4, lua_string!($name)); // serverstat

            lua.push_function(export::r#async::$func);
            lua.set_field(-3, lua_string!($name)); // async

            lua.push_function(realtime::$func);
            lua.set_field(-2, lua_string!($name)); // realtime
        };
    }

    macro_rules! push_realtime_func {
        ($func:ident, $name:literal) => {
            lua.push_function(realtime::$func);
            lua.set_field(-2, lua_string!($name)); // realtime
        };
    }

    push_realtime_api_func!(process_cpu_usage, "ProcessCPUUsage");
    push_realtime_api_func!(process_memory_usage, "ProcessMemoryUsage");
    push_realtime_api_func!(system_cpu_usage, "SystemCPUUsage");
    push_realtime_api_func!(system_memory_usage, "SystemMemoryUsage");
    push_realtime_api_func!(system_available_memory, "SystemAvailableMemory");

    push_api_func!(all, "All");
    push_api_func!(all_system, "AllSystem");
    push_api_func!(all_process, "AllProcess");
    push_api_func!(system_total_memory, "SystemTotalMemory");
    push_api_func!(logical_cpus, "LogicalCPUs");
    push_api_func!(physical_cpus, "PhysicalCPUs");

    push_realtime_func!(start, "Start");
    push_realtime_func!(stop, "Stop");
    push_realtime_func!(all, "All");
    push_realtime_func!(all, "AllCopy");
    push_realtime_func!(all_system, "AllSystem");
    push_realtime_func!(all_system, "AllSystemCopy");
    push_realtime_func!(all_process, "AllProcess");
    push_realtime_func!(all_process, "AllProcessCopy");
    push_realtime_func!(set_interval, "SetInterval");

    lua.set_field(-3, lua_string!("realtime")); // serverstat.realtime = realtime
    lua.set_field(-2, lua_string!("async")); // serverstat.async = async
    lua.set_global(lua_string!("serverstat"));

    crate::lua_metrics::install(lua);

    0
}

#[gmod13_close]
unsafe fn gmod13_close(lua: LuaState) -> i32 {
    crate::prometheus::stop();
    crate::lua_metrics::uninstall(lua);

    lua.get_global(lua_string!("hook"));
    lua.get_field(-1, lua_string!("Remove"));
    lua.push_string("Tick");
    lua.push_string("gmsv_serverstat");
    lua.call(2, 0);
    lua.pop();

    lua.get_global(lua_string!("timer"));
    lua.get_field(-1, lua_string!("Remove"));
    lua.push_string("gmsv_serverstat_realtime");
    lua.call(1, 0);
    lua.pop();

    0
}

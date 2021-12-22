

extern "C" {
    pub fn vl_get_time() -> ::std::os::raw::c_ulong;
    pub fn vl_set_time(t: ::std::os::raw::c_ulong);
    pub fn vl_init();
    pub fn vl_eval();
    pub fn vl_finalize();
    pub fn vl_got_finish() -> bool;
    pub fn get_signal_by_name();
    pub fn vl_get_root_handle() -> usize;
}
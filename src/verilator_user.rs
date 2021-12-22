

extern "C" {
    pub(crate) fn vl_get_time() -> ::std::os::raw::c_ulong;
    pub(crate) fn vl_set_time(t: ::std::os::raw::c_ulong);
    pub(crate) fn vl_init();
    pub(crate) fn vl_eval();
    pub(crate) fn vl_finalize();
    pub(crate) fn vl_got_finish() -> bool;
    pub(crate) fn get_signal_by_name();
    pub(crate) fn vl_get_root_handle() -> usize;
    pub(crate) fn vl_print_scope(name: *const ::std::os::raw::c_char);
}
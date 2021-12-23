

extern "C" {
    pub(crate) fn vl_get_time() -> ::std::os::raw::c_ulong;
    pub(crate) fn vl_set_time(t: ::std::os::raw::c_ulong);
    pub(crate) fn vl_init();
    pub(crate) fn vl_eval();
    pub(crate) fn vl_finalize();
    pub(crate) fn vl_got_finish() -> bool;
    pub(crate) fn vl_get_root_scope() -> usize;
    pub(crate) fn vl_print_scope(name: *const ::std::os::raw::c_char);
    pub(crate) fn vl_print_scopes();
    pub(crate) fn vl_get_root_scope_name() -> *const ::std::os::raw::c_char;
    pub(crate) fn vl_get_scope_name(handle: usize) -> *const ::std::os::raw::c_char;
    pub(crate) fn vl_get_scope_by_name(name: *const ::std::os::raw::c_char) -> usize;
    pub(crate) fn vl_get_var_by_name(scope: usize, name: *const ::std::os::raw::c_char) -> usize;
    pub(crate) fn vl_get_var_name(handle: usize) -> *const ::std::os::raw::c_char;
    pub(crate) fn vl_get_var_type(var: usize) -> u8;
    pub(crate) fn vl_set_var_u8(var: usize, val: u8);
    pub(crate) fn vl_set_var_u16(var: usize, val: u16);
    pub(crate) fn vl_set_var_u32(var: usize, val: u32);
    pub(crate) fn vl_set_var_u64(var: usize, val: u64);
    pub(crate) fn vl_get_var_u8(var: usize) -> u8;
    pub(crate) fn vl_get_var_u16(var: usize) -> u16;
    pub(crate) fn vl_get_var_u32(var: usize) -> u32;
    pub(crate) fn vl_get_var_u64(var: usize) -> u64;
    
// extern "C" uint8_t vl_get_var_type(uintptr_t var) {
}
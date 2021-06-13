bindgen ../src/vpi_user.h -o ../src/vpi_user.rs --no-layout-tests --with-derive-default \
    --allowlist-var cbValueChange \
    --allowlist-var cbStartOfSimulation \
    --allowlist-var cbEndOfSimulation \
    --allowlist-var cbAfterDelay \
    --allowlist-var cbReadWriteSynch \
    --allowlist-var cbReadOnlySynch \
    --allowlist-var vpiSimTime \
    --allowlist-var vpiTimePrecision \
    --allowlist-var vpiSuppressTime \
    --allowlist-var vpiSize \
    --allowlist-var vpiName \
    --allowlist-var vpiFullName \
    --allowlist-var vpiDefName \
    --allowlist-var vpiType \
    --allowlist-var vpiSigned \
    --allowlist-var vpiBinStrVal \
    --allowlist-var vpiIntVal \
    --allowlist-var vpiStringVal \
    --allowlist-var vpiSuppressVal \
    --allowlist-var vpiInertialDelay \
    --allowlist-var vpiArray \
    --allowlist-var vpiIntegerVar \
    --allowlist-var vpiRealVar \
    --allowlist-var vpiNet \
    --allowlist-var vpiNetBit \
    --allowlist-var vpiReg \
    --allowlist-var vpiRegBit \
    --allowlist-var vpiPort \
    --allowlist-var vpiPortBit \
    --allowlist-var vpiMemoryWord \
    --allowlist-var vpiModule \
    \
    --allowlist-function vpi_register_cb \
    --allowlist-function vpi_remove_cb \
    --allowlist-function vpi_handle_by_name \
    --allowlist-function vpi_put_value \
    --allowlist-function vpi_get_value \
    --allowlist-function vpi_get_time \
    --allowlist-function vpi_get_str \
    --allowlist-function vpi_get \
    --allowlist-function vpi_iterate \
    --allowlist-function vpi_scan \
    --allowlist-function vpi_free_object \
    --allowlist-function vpi_printf
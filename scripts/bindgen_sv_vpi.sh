bindgen ../src/sv_vpi_user.h -o ../src/sv_vpi_user.rs --no-layout-tests --with-derive-default \
    --allowlist-var vpiIntVar \
    --allowlist-var vpiLongIntVar \
    --allowlist-var vpiBitVar \
    --allowlist-var vpiShortRealVar
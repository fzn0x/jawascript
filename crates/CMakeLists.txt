# Builds the Rust half of jawac with Cargo. The jawac crate is a static
# library; tools/jawac links it with the C++ backend.

find_program(CARGO cargo REQUIRED)
find_program(RUSTC rustc REQUIRED)

set(JAWAC_CARGO_TARGET_DIR ${PROJECT_BINARY_DIR}/cargo)
set(JAWAC_CXXBRIDGE_DIR ${PROJECT_BINARY_DIR}/cxxbridge CACHE INTERNAL
  "Where the cxx bridge headers are written for the C++ side")
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
  set(cargo_profile debug)
  set(cargo_flags "")
else()
  set(cargo_profile release)
  set(cargo_flags --release)
endif()
set(JAWAC_RUST_LIB
  ${JAWAC_CARGO_TARGET_DIR}/${cargo_profile}/${CMAKE_STATIC_LIBRARY_PREFIX}jawac${CMAKE_STATIC_LIBRARY_SUFFIX})

# jawac links programs with the same C compiler, the runtime and its
# collector; bake those in. Arguments are `|`-separated.
string(REPLACE ";" "|" link_args "${JAWA_LINK_ARGS}")

add_custom_target(jawac-rust
  COMMAND ${CMAKE_COMMAND} -E env
    CARGO_TARGET_DIR=${JAWAC_CARGO_TARGET_DIR}
    JAWA_CXXBRIDGE_DIR=${JAWAC_CXXBRIDGE_DIR}
    CC=${CMAKE_C_COMPILER}
    CXX=${CMAKE_CXX_COMPILER}
    JAWA_DEFAULT_CC=${CMAKE_C_COMPILER}
    JAWA_RUNTIME_LIB=$<TARGET_FILE:jawa_rt>
    JAWA_LINK_ARGS=${link_args}
    ${CARGO} build --package jawac ${cargo_flags}
  WORKING_DIRECTORY ${PROJECT_SOURCE_DIR}
  BYPRODUCTS
    ${JAWAC_RUST_LIB}
    ${JAWAC_CXXBRIDGE_DIR}/rust/cxx.h
    ${JAWAC_CXXBRIDGE_DIR}/jawac/src/bridge.rs.h
  COMMENT "Building the Rust frontend with Cargo"
  USES_TERMINAL
  VERBATIM)
add_dependencies(jawac-rust jawa_rt)

# The system libraries a Rust static library needs, as rustc reports them.
set(probe_dir ${CMAKE_CURRENT_BINARY_DIR}/native-libs-probe)
file(WRITE ${probe_dir}/probe.rs "")
execute_process(
  COMMAND ${RUSTC} --crate-type staticlib --crate-name probe
    --print native-static-libs -o ${probe_dir}/probe.lib ${probe_dir}/probe.rs
  ERROR_VARIABLE probe_output
  OUTPUT_QUIET)
string(REGEX MATCH "native-static-libs: ([^\r\n]*)" _ "${probe_output}")
separate_arguments(RUST_NATIVE_LIBS NATIVE_COMMAND "${CMAKE_MATCH_1}")
message(STATUS "Rust static libraries need: ${RUST_NATIVE_LIBS}")

add_library(jawac_rust STATIC IMPORTED GLOBAL)
set_target_properties(jawac_rust PROPERTIES IMPORTED_LOCATION ${JAWAC_RUST_LIB})
add_dependencies(jawac_rust jawac-rust)
# The Rust library calls into the bridge, and the bridge into cxx's support
# code inside the Rust library; CMake repeats the pair on the link line.
target_link_libraries(jawac_rust INTERFACE JawaBridge ${RUST_NATIVE_LIBS})

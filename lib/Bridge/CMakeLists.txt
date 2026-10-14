# The C++ side of the cxx bridge. It includes headers cxx generates while
# Cargo builds the Rust frontend.
include_directories(${JAWAC_CXXBRIDGE_DIR})

add_mlir_library(JawaBridge
  Bridge.cpp

  ADDITIONAL_HEADER_DIRS
  ${PROJECT_SOURCE_DIR}/include/jawa/Bridge

  DEPENDS
  jawac-rust

  LINK_LIBS PUBLIC
  JawaPipeline
  jawac_rust
)

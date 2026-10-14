add_mlir_dialect_library(JawaTransforms
  FlattenCFG.cpp
  FuseConcat.cpp

  # Internal to jawac; `cmake --install` installs only jawac and the runtime.
  DISABLE_INSTALL

  ADDITIONAL_HEADER_DIRS
  ${PROJECT_SOURCE_DIR}/include/jawa/Dialect/Jawa/Transforms

  DEPENDS
  JawaPassIncGen

  LINK_LIBS PUBLIC
  JawaDialect
  MLIRControlFlowDialect
  MLIRFuncDialect
  MLIRPass
  MLIRTransformUtils
)

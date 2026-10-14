add_mlir_dialect_library(JawaTransforms
  FlattenCFG.cpp
  FuseConcat.cpp

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
